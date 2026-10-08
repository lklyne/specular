//! Auto-layout groups (ADR 0015): a group with `managedLayout` packs its
//! direct members into a row or a column.
//!
//! The members' rects are outputs. [`reflow`] is the one writer of them,
//! and `group_fit::then_fit` adds its commands to every step that changes a
//! managed group's members, their sizes, their order or its own fields, so
//! one action is one undo step and the document always holds the resolved
//! rects.
//!
//! The layout sequence is the order of the members in the stack: there is
//! no second list to keep in step with it.

pub(crate) mod act;
pub(crate) mod drag;
pub(crate) mod handles;
pub(crate) mod row;

use glam::DVec2;
use specular_doc::{Command, Document, Entity, EntityId, Group, ItemId, Kind, LayoutMode, Rect};

use crate::{grid, groups, scope, stack_order, verbs};

/// The gap a managed group packs with when it names none.
pub(crate) const DEFAULT_GAP: f64 = 80.0;

/// The axis a line is packed along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    /// Left to right: a row.
    X,
    /// Top to bottom: a column.
    Y,
}

impl Axis {
    pub(crate) const fn other(self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::X,
        }
    }

    /// The leading edge (left or top) of `rect` along this axis.
    pub(crate) const fn lead(self, rect: Rect) -> f64 {
        match self {
            Self::X => rect.x,
            Self::Y => rect.y,
        }
    }

    /// The size of `rect` along this axis.
    pub(crate) const fn size(self, rect: Rect) -> f64 {
        match self {
            Self::X => rect.width,
            Self::Y => rect.height,
        }
    }

    /// The far edge of `rect` along this axis.
    pub(crate) const fn trail(self, rect: Rect) -> f64 {
        self.lead(rect) + self.size(rect)
    }

    /// The middle of `rect` along this axis.
    pub(crate) const fn centre(self, rect: Rect) -> f64 {
        self.lead(rect) + self.size(rect) / 2.0
    }

    /// `point`'s coordinate along this axis.
    pub(crate) const fn of(self, point: DVec2) -> f64 {
        match self {
            Self::X => point.x,
            Self::Y => point.y,
        }
    }

    /// The point `along` this axis and `across` it.
    pub(crate) const fn point(self, along: f64, across: f64) -> DVec2 {
        match self {
            Self::X => DVec2::new(along, across),
            Self::Y => DVec2::new(across, along),
        }
    }
}

/// The axis `group` packs its members along, or `None` when it does not
/// manage them. A grid is not a managed mode: Electron has none.
pub(crate) fn line_axis(group: &Group) -> Option<Axis> {
    if group.managed_layout != Some(true) {
        return None;
    }
    match group.layout_mode? {
        LayoutMode::Row => Some(Axis::X),
        LayoutMode::Column => Some(Axis::Y),
        LayoutMode::Freeform | LayoutMode::Grid => None,
    }
}

/// The gap `group` packs with.
pub(crate) fn gap_of(group: &Group) -> f64 {
    group.layout_gap.unwrap_or(DEFAULT_GAP)
}

/// The group fields of `entity`, when it is a group.
pub(crate) fn group_of(entity: &Entity) -> Option<&Group> {
    match &entity.kind {
        Kind::Group(group) => Some(group),
        Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Drawing(_) | Kind::Shape(_) => None,
    }
}

/// A managed line: the group, the axis it packs along, its gap, and its
/// members in layout order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    pub(crate) group: EntityId,
    pub(crate) axis: Axis,
    pub(crate) gap: f64,
    pub(crate) members: Vec<(EntityId, Rect)>,
}

/// The managed line `id` is, if it is one.
pub(crate) fn line(document: &Document, id: &EntityId) -> Option<Line> {
    let group = group_of(document.entity(id)?)?;
    Some(Line {
        group: id.clone(),
        axis: line_axis(group)?,
        gap: gap_of(group),
        members: (document.children(id))
            .map(|child| (child.id.clone(), child.rect))
            .collect(),
    })
}

/// The managed line `child` is a direct member of.
pub(crate) fn line_holding(document: &Document, child: &EntityId) -> Option<Line> {
    line(document, document.entity(child)?.parent.as_ref()?)
}

/// The top-left of each of `rects` packed in the order given along `axis`
/// from `origin`, `gap` apart. Every one sits at the origin across the axis.
pub(crate) fn packed(rects: &[Rect], gap: f64, origin: DVec2, axis: Axis) -> Vec<DVec2> {
    let across = axis.other().of(origin);
    let mut cursor = axis.of(origin);
    rects
        .iter()
        .map(|rect| {
            let at = axis.point(cursor, across);
            cursor += axis.size(*rect) + gap;
            at
        })
        .collect()
}

/// The commands that put the members of the managed group `id` where its
/// layout has them: packed in stack order from their least corner, which is
/// put on the grid. A member group travels with everything in it. Empty for
/// a group that manages nothing, and for one already laid out.
pub(crate) fn reflow(document: &Document, id: &EntityId) -> Vec<Command> {
    let Some(line) = line(document, id) else {
        return Vec::new();
    };
    let rects: Vec<Rect> = line.members.iter().map(|(_, rect)| *rect).collect();
    let least = |axis: Axis| {
        (rects.iter())
            .map(|rect| axis.lead(*rect))
            .fold(f64::INFINITY, f64::min)
    };
    let origin = DVec2::new(grid::snap(least(Axis::X)), grid::snap(least(Axis::Y)));
    let is_member = |entity: &EntityId| line.members.iter().any(|(member, _)| member == entity);
    (line.members.iter())
        .zip(packed(&rects, line.gap, origin, line.axis))
        .flat_map(|((member, rect), to)| {
            let by = to - DVec2::new(rect.x, rect.y);
            if by == DVec2::ZERO {
                return Vec::new();
            }
            // What is hooked to a member page and is a member itself has its
            // own slot.
            let carried: Vec<EntityId> = scope::operands(document, std::slice::from_ref(member))
                .into_iter()
                .filter(|entity| entity == member || !is_member(entity))
                .collect();
            verbs::translate_commands(document, &carried, by)
        })
        .collect()
}

/// The order that puts the direct members of `group` in the sequence
/// `members`, each with everything inside it, where the first of them was.
/// `None` when that is the order already.
pub(crate) fn sequenced(
    document: &Document,
    group: &EntityId,
    members: &[EntityId],
) -> Option<Command> {
    let order = document.order();
    let block = |member: &EntityId| {
        let mut items = vec![ItemId::Entity(member.clone())];
        items.extend(groups::descendants(document, member));
        let mut placed: Vec<ItemId> = (order.iter())
            .filter(|item| items.contains(item))
            .cloned()
            .collect();
        // Whatever the stack had missing still travels.
        placed.extend(items.into_iter().filter(|item| !order.contains(item)));
        placed
    };
    let moved: Vec<ItemId> = (document.children(group))
        .map(|child| child.id.clone())
        .filter(|child| !members.contains(child))
        .chain(members.iter().cloned())
        .flat_map(|member| block(&member))
        .filter(|item| order.contains(item))
        .collect();
    let first = order.iter().position(|item| moved.contains(item))?;
    let mut next: Vec<ItemId> = (order.iter())
        .filter(|item| !moved.contains(item))
        .cloned()
        .collect();
    let at = order[..first]
        .iter()
        .filter(|item| !moved.contains(item))
        .count();
    next.splice(at..at, moved);
    let next = stack_order::enforce_contiguity(&next, &stack_order::document_runs(document));
    (next != order).then_some(Command::SetOrder(next))
}

/// The commands that make `group` a managed row or column: along `axis`,
/// or with `None` the one its members are spread along. The sequence is
/// their order along it now, so nothing jumps, and `gap` is kept when
/// given. The reflow follows in the step these run in. `None` for an entity
/// that is not a group.
pub(crate) fn manage(
    document: &Document,
    group: &EntityId,
    axis: Option<Axis>,
    gap: Option<f64>,
) -> Option<Command> {
    let entity = document.entity(group)?;
    let fields = group_of(entity)?;
    let mut members: Vec<(EntityId, Rect)> = (document.children(group))
        .map(|child| (child.id.clone(), child.rect))
        .collect();
    let rects: Vec<Rect> = members.iter().map(|(_, rect)| *rect).collect();
    let axis = axis.unwrap_or_else(|| row::dominant_axis(&rects));
    members.sort_by(|a, b| axis.lead(a.1).total_cmp(&axis.lead(b.1)));
    let ids: Vec<EntityId> = members.into_iter().map(|(id, _)| id).collect();
    let managed = Group {
        managed_layout: Some(true),
        layout_mode: Some(act::mode_of(axis)),
        layout_gap: gap.and_then(normal_gap).or(fields.layout_gap),
        ..fields.clone()
    };
    let mut commands = Vec::new();
    if managed != *fields {
        commands.push(Command::SetKind {
            id: group.clone(),
            kind: Box::new(Kind::Group(managed)),
        });
    }
    commands.extend(sequenced(document, group, &ids));
    Some(Command::Batch(commands))
}

/// A gap as a group stores it: whole and not negative. `None` for one that
/// is not a number.
pub(crate) fn normal_gap(gap: f64) -> Option<f64> {
    gap.is_finite().then(|| grid::round(gap).max(0.0))
}

/// The command that gives the group `id` these layout fields, or `None`
/// when it has them or is not a group. The reflow follows in the step.
pub(crate) fn set_fields(
    document: &Document,
    id: &EntityId,
    change: impl FnOnce(&mut Group),
) -> Option<Command> {
    let fields = group_of(document.entity(id)?)?;
    let mut next = fields.clone();
    change(&mut next);
    (next != *fields).then(|| Command::SetKind {
        id: id.clone(),
        kind: Box::new(Kind::Group(next)),
    })
}

/// The command that moves `child` to `to` in the sequence of the managed
/// group `group`. `None` when it is there already, or is not a member of a
/// managed group.
pub(crate) fn reordered(
    document: &Document,
    group: &EntityId,
    child: &EntityId,
    to: usize,
) -> Option<Command> {
    let line = line(document, group)?;
    let mut members: Vec<EntityId> = line.members.into_iter().map(|(id, _)| id).collect();
    let from = members.iter().position(|member| member == child)?;
    let to = to.min(members.len() - 1);
    if from == to {
        return None;
    }
    let moving = members.remove(from);
    members.insert(to, moving);
    sequenced(document, group, &members)
}
