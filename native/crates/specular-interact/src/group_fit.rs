//! A group's rect follows its members: whatever a step does to a member's
//! rect or membership, the groups above it are refitted to the union of
//! their direct members plus padding, in the same step. A managed group
//! lays its members out first (`layout`).
//!
//! Only groups the step touched are refitted, so a hand-sized group in a
//! loaded file stays as it is until something in it changes. A group that
//! moves with all of its members, was itself resized, or is created by the
//! step (a copy of a group keeps its size) is not
//! touched by that. An empty group keeps its rect.

use specular_doc::{Command, Document, EntityId, Kind, Rect};

use crate::{geometry, layout};

/// Room kept between a group's edge and the members it wraps.
pub(crate) const PADDING: f64 = 24.0;
/// How far two rects may differ and still be a translation of one another.
const TOLERANCE: f64 = 1e-6;

/// The commands refitting `groups` (inside-out, so a nested group is fitted
/// before the group around it), applied to `document` as they are found.
/// Each comes with the command that undoes it.
pub(crate) fn fit_in_place(
    document: &mut Document,
    groups: &[EntityId],
) -> Vec<(Command, Command)> {
    settle_in_place(document, groups, &[])
}

/// As [`fit_in_place`], with the groups of `relaid` that manage their
/// members laying them out first (see `layout`), and then fitted whether or
/// not they are among `groups`.
fn settle_in_place(
    document: &mut Document,
    groups: &[EntityId],
    relaid: &[EntityId],
) -> Vec<(Command, Command)> {
    let mut ordered: Vec<&EntityId> = groups.iter().collect();
    ordered.extend(relaid.iter().filter(|id| !groups.contains(id)));
    ordered.sort_by_key(|id| std::cmp::Reverse(document.ancestors(id).count()));
    let mut applied = Vec::new();
    for id in ordered {
        let mut moved = false;
        if relaid.contains(id) {
            for command in layout::reflow(document, id) {
                if let Ok(undo) = document.apply(command.clone()) {
                    applied.push((command, undo));
                    moved = true;
                }
            }
        }
        if !moved && !groups.contains(id) {
            continue;
        }
        let Some(rect) = (document.children(id))
            .map(|child| child.rect)
            .reduce(geometry::union)
        else {
            continue;
        };
        let fitted = Rect::new(
            rect.x - PADDING,
            rect.y - PADDING,
            rect.width + PADDING * 2.0,
            rect.height + PADDING * 2.0,
        );
        if document.entity(id).is_none_or(|group| group.rect == fitted) {
            continue;
        }
        let command = Command::SetRect {
            id: id.clone(),
            rect: fitted,
        };
        if let Ok(undo) = document.apply(command.clone()) {
            applied.push((command, undo));
        }
    }
    applied
}

/// What a step touches that a group's rect or layout follows.
#[derive(Default)]
struct Touched {
    /// The entities whose rect or membership changes.
    ids: Vec<EntityId>,
    /// Whether anything is put in a group.
    joins: bool,
    /// The groups the step creates.
    new_groups: Vec<EntityId>,
    /// The entities whose kind fields change: a group's layout among them.
    kinds: Vec<EntityId>,
    /// Whether the stack order, which is a managed group's sequence,
    /// changes.
    reordered: bool,
}

fn touched(step: &Command, found: &mut Touched) {
    let Touched {
        ids,
        joins,
        new_groups,
        kinds,
        reordered,
    } = found;
    match step {
        Command::SetRect { id, .. } | Command::RemoveEntity(id) => ids.push(id.clone()),
        Command::SetParent { id, parent } => {
            ids.push(id.clone());
            *joins |= parent.is_some();
        }
        Command::InsertEntity { entity, .. } => {
            ids.push(entity.id.clone());
            *joins |= entity.parent.is_some();
            if matches!(entity.kind, Kind::Group(_)) {
                new_groups.push(entity.id.clone());
            }
        }
        Command::Batch(commands) => {
            for command in commands {
                touched(command, found);
            }
        }
        Command::SetKind { id, .. } => kinds.push(id.clone()),
        Command::SetOrder(_) => *reordered = true,
        Command::SetLabel { .. }
        | Command::SetNote { .. }
        | Command::SetAnchor { .. }
        | Command::InsertEdge { .. }
        | Command::RemoveEdge(_)
        | Command::ReplaceEdge(_)
        | Command::InsertAnnotation { .. }
        | Command::RemoveAnnotation(_)
        | Command::ReplaceAnnotation(_) => {}
    }
}

/// Whether `group` and all of its members moved by one offset, so nothing
/// changed between them.
fn travelled_whole(document: &Document, before: &[(EntityId, Rect)], group: &EntityId) -> bool {
    let rect_before = |id: &EntityId| before.iter().find(|(known, _)| known == id).map(|e| e.1);
    let (Some(now), Some(was)) = (document.entity(group), rect_before(group)) else {
        return false;
    };
    let same = |a: f64, b: f64| (a - b).abs() < TOLERANCE;
    let delta = (now.rect.x - was.x, now.rect.y - was.y);
    if !same(now.rect.width, was.width)
        || !same(now.rect.height, was.height)
        || (same(delta.0, 0.0) && same(delta.1, 0.0))
    {
        return false;
    }
    let mut members = document.children(group).peekable();
    members.peek().is_some()
        && members.all(|child| {
            rect_before(&child.id).is_some_and(|rect| {
                same(child.rect.x - rect.x, delta.0)
                    && same(child.rect.y - rect.y, delta.1)
                    && same(child.rect.width, rect.width)
                    && same(child.rect.height, rect.height)
            })
        })
}

/// `step` followed by the layout of every managed group it touched and
/// the refit of every group it touched. `skip` are the groups the step
/// moves or resizes itself, which it leaves as they are.
pub(crate) fn then_fit(document: &mut Document, step: Command, skip: &[EntityId]) -> Command {
    let mut found = Touched::default();
    touched(&step, &mut found);
    let Touched {
        ids,
        joins,
        new_groups,
        kinds,
        reordered,
    } = found;
    let mut groups: Vec<EntityId> = Vec::new();
    for group in ids.iter().flat_map(|id| document.ancestors(id)) {
        if !groups.contains(&group.id) {
            groups.push(group.id.clone());
        }
    }
    if groups.is_empty() && !joins && kinds.is_empty() && !reordered {
        return step;
    }
    let before: Vec<(EntityId, Rect)> = groups
        .iter()
        .flat_map(|id| {
            let group = document.entity(id).map(|group| (id.clone(), group.rect));
            let members = document
                .children(id)
                .map(|child| (child.id.clone(), child.rect));
            group.into_iter().chain(members).collect::<Vec<_>>()
        })
        .collect();
    let Ok(undo) = document.apply(step.clone()) else {
        return step;
    };
    for group in ids.iter().flat_map(|id| document.ancestors(id)) {
        if !groups.contains(&group.id) {
            groups.push(group.id.clone());
        }
    }
    let settled = |id: &EntityId| skip.contains(id) || travelled_whole(document, &before, id);
    // A managed group lays its members out when they, their order or its
    // own fields change, a new one included.
    let managed = |id: &EntityId| layout::line(document, id).is_some();
    let relaid: Vec<EntityId> = if reordered {
        (document.entities())
            .map(|entity| entity.id.clone())
            .filter(|id| managed(id) && !skip.contains(id))
            .collect()
    } else {
        (groups.iter().chain(&kinds))
            .filter(|id| managed(id) && !settled(id))
            .cloned()
            .collect()
    };
    groups.retain(|id| document.entity(id).is_some() && !new_groups.contains(id) && !settled(id));
    let fits = settle_in_place(document, &groups, &relaid);
    for (_, undo) in fits.iter().rev() {
        if let Err(error) = document.apply(undo.clone()) {
            tracing::warn!("trial refit not taken back: {error}");
        }
    }
    if let Err(error) = document.apply(undo) {
        tracing::warn!("trial step not taken back: {error}");
    }
    if fits.is_empty() {
        return step;
    }
    let mut commands = vec![step];
    commands.extend(fits.into_iter().map(|(command, _)| command));
    Command::Batch(commands)
}
