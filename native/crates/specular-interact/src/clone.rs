//! Copies of the selection, for Option-drag and duplicate.

use std::collections::HashMap;

use glam::DVec2;
use specular_doc::{Command, Edge, EdgeId, Entity, EntityId, ItemId};

use crate::live::Start;
use crate::{App, SelectionScope};

/// A command that adds copies, and what to select once it has run.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Copies {
    /// Inserts every copy, in front of everything, as one step.
    pub(crate) command: Command,
    /// The copies of the selected entities, in selection order.
    pub(crate) members: Vec<ItemId>,
}

/// Copies of everything in `scope`, `delta` away from the originals, with
/// fresh ids. `None` when there is nothing to copy.
///
/// The copies keep their order in the stack and go in front. A copy stays in
/// its group unless the group is copied too, in which case it joins the
/// group's copy. An edge is copied when both its ends are. A copy hooked to a
/// page stays hooked only when that page is copied with it.
pub(crate) fn copies(app: &mut App, scope: &SelectionScope, delta: DVec2) -> Option<Copies> {
    let originals: Vec<Entity> = (app.document.entities())
        .filter(|entity| scope.operands.contains(&entity.id))
        .cloned()
        .collect();
    if originals.is_empty() {
        return None;
    }
    let ids: HashMap<EntityId, EntityId> = originals
        .iter()
        .map(|entity| (entity.id.clone(), EntityId::new(app.fresh_id())))
        .collect();
    let edges: Vec<Edge> = (app.document.edges())
        .filter(|edge| ids.contains_key(&edge.from) && ids.contains_key(&edge.to))
        .cloned()
        .collect();
    let edge_ids: HashMap<EdgeId, EdgeId> = edges
        .iter()
        .map(|edge| (edge.id.clone(), EdgeId::new(app.fresh_id())))
        .collect();

    let renamed = |id: &EntityId| ids.get(id).cloned();
    let mut commands = Vec::new();
    // Entities and edges share the stack, so the copies are walked in its
    // order and each goes in front of the one before.
    for item in app.document.order() {
        let at = app.document.stack_len() + commands.len();
        match item {
            ItemId::Entity(id) => {
                let (Some(new_id), Some(entity)) = (renamed(id), app.document.entity(id)) else {
                    continue;
                };
                let (rect, kind) = Start::of(entity).moved(delta);
                let anchor = entity.anchor.clone().and_then(|anchor| {
                    Some(specular_doc::PageAnchor {
                        page_id: renamed(&anchor.page_id)?,
                        ..anchor
                    })
                });
                let copy = Entity {
                    id: new_id,
                    rect,
                    kind: kind.unwrap_or_else(|| entity.kind.clone()),
                    parent: (entity.parent.as_ref())
                        .map(|parent| renamed(parent).unwrap_or_else(|| parent.clone())),
                    anchor,
                    ..entity.clone()
                };
                commands.push(Command::InsertEntity {
                    entity: Box::new(copy),
                    at,
                });
            }
            ItemId::Edge(id) => {
                let (Some(new_id), Some(edge)) = (edge_ids.get(id), app.document.edge(id)) else {
                    continue;
                };
                let (Some(from), Some(to)) = (renamed(&edge.from), renamed(&edge.to)) else {
                    continue;
                };
                let copy = Edge {
                    id: new_id.clone(),
                    from,
                    to,
                    ..edge.clone()
                };
                commands.push(Command::InsertEdge {
                    edge: Box::new(copy),
                    at,
                });
            }
        }
    }
    let members = (scope.members.iter())
        .filter_map(renamed)
        .map(ItemId::Entity)
        .collect();
    Some(Copies {
        command: Command::Batch(commands),
        members,
    })
}
