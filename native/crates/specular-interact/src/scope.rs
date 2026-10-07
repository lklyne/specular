//! [`SelectionScope`]: what the selection means to a gesture or a verb
//! (ADR 0034). Move, resize, copy, delete and the selection outline all read
//! this one resolution, so they cannot disagree about a group.

use specular_doc::{Document, Entity, EntityId, Kind, Rect};

use crate::{App, geometry};

/// The selection resolved for gestures and verbs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectionScope {
    /// The selected entities as selected. A group is one member. What
    /// reparenting, grouping and ungrouping act on.
    pub members: Vec<EntityId>,
    /// The flat set a gesture moves, copies or deletes: each member, every
    /// descendant of a member group, and whatever is hooked to a page among
    /// them.
    pub operands: Vec<EntityId>,
    /// The union of the operands' rects in canvas space. A group counts with
    /// its own rect, so the bounds wrap its border. `None` when there are no
    /// operands.
    pub bounds: Option<Rect>,
}

impl SelectionScope {
    /// Whether a press on `id` is a press on the selection: it is a member,
    /// a descendant of a member group, or hooked to a member page. Such a
    /// press keeps the selection, so a drag from it takes everything.
    pub fn holds(&self, id: &EntityId) -> bool {
        self.operands.contains(id)
    }
}

impl App {
    /// The selection resolved for gestures and verbs. Edges are not part of
    /// it: they follow the entities they connect.
    pub fn selection_scope(&self) -> SelectionScope {
        let document = &self.document;
        let members: Vec<EntityId> = self
            .session
            .selection
            .entities()
            .filter(|id| document.entity(id).is_some())
            .cloned()
            .collect();
        let operands = operands(document, &members);
        let bounds = operands
            .iter()
            .filter_map(|id| document.entity(id))
            .map(|entity| entity.rect)
            .reduce(geometry::union);
        SelectionScope {
            members,
            operands,
            bounds,
        }
    }
}

/// `members` with every group expanded to its descendants, then everything
/// hooked to a page in that set.
fn operands(document: &Document, members: &[EntityId]) -> Vec<EntityId> {
    let mut out: Vec<EntityId> = Vec::new();
    // A stack, so a parent cycle in a hand-edited file ends at the first
    // repeat.
    let mut pending: Vec<&EntityId> = members.iter().rev().collect();
    while let Some(id) = pending.pop() {
        if out.contains(id) {
            continue;
        }
        out.push(id.clone());
        if document.entity(id).is_some_and(is_group) {
            let children: Vec<&EntityId> = document.children(id).map(|child| &child.id).collect();
            pending.extend(children.into_iter().rev());
        }
    }
    let hooked: Vec<EntityId> = document
        .entities()
        .filter(|entity| {
            entity
                .anchor
                .as_ref()
                .is_some_and(|anchor| out.contains(&anchor.page_id))
                && !out.contains(&entity.id)
        })
        .map(|entity| entity.id.clone())
        .collect();
    out.extend(hooked);
    out
}

pub(crate) const fn is_group(entity: &Entity) -> bool {
    match entity.kind {
        Kind::Group(_) => true,
        Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Drawing(_) | Kind::Shape(_) => false,
    }
}
