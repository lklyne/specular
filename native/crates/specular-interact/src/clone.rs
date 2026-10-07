//! Copies of entities and the edges between them, for Option-drag, duplicate
//! and paste.

use std::collections::HashMap;

use glam::DVec2;
use specular_doc::{Command, Document, Edge, EdgeId, Entity, EntityId, ItemId};

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

/// What a copy does about a group that is not copied with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Orphan {
    /// It stays in the group: the copy sits beside its original.
    Stays,
    /// It has no group: the copy came from another document.
    Leaves,
}

/// Copies of everything in `scope`, `delta` away from the originals, with
/// fresh ids. `None` when there is nothing to copy.
///
/// The copies keep their order in the stack and go in front. A copy stays in
/// its group unless the group is copied too, in which case it joins the
/// group's copy. An edge is copied when both its ends are. A copy hooked to a
/// page stays hooked only when that page is copied with it.
pub(crate) fn copies(app: &mut App, scope: &SelectionScope, delta: DVec2) -> Option<Copies> {
    let copied = |id: &EntityId| scope.operands.contains(id);
    let ids = fresh_ids(app, |app| needed(&app.document, copied));
    let at = app.document.stack_len();
    let (commands, renamed) = insertions(&app.document, copied, ids, at, delta, Orphan::Stays);
    if commands.is_empty() {
        return None;
    }
    let members = (scope.members.iter())
        .filter_map(|id| renamed.get(id).cloned())
        .map(ItemId::Entity)
        .collect();
    Some(Copies {
        command: Command::Batch(commands),
        members,
    })
}

/// As many ids as `count` asks for, none of them in `app`'s document.
pub(crate) fn fresh_ids(app: &mut App, count: impl FnOnce(&App) -> usize) -> Vec<String> {
    let count = count(app);
    (0..count).map(|_| app.fresh_id()).collect()
}

/// How many ids [`insertions`] takes for the entities of `source` that
/// `copied` accepts: one each, and one for each edge between two of them.
pub(crate) fn needed(source: &Document, copied: impl Fn(&EntityId) -> bool) -> usize {
    let entities = source.entities().filter(|entity| copied(&entity.id));
    let edges = (source.edges()).filter(|edge| copied(&edge.from) && copied(&edge.to));
    entities.count() + edges.count()
}

/// The commands that insert copies of the entities of `source` that `copied`
/// accepts, `delta` away, from stack position `at` up, and the id each
/// original's copy got. `ids` are the copies' ids: the entities' first, in
/// stack order, then the edges'.
pub(crate) fn insertions(
    source: &Document,
    copied: impl Fn(&EntityId) -> bool,
    ids: Vec<String>,
    at: usize,
    delta: DVec2,
    orphan: Orphan,
) -> (Vec<Command>, HashMap<EntityId, EntityId>) {
    let mut ids = ids.into_iter();
    let entity_ids: HashMap<EntityId, EntityId> = (source.entities())
        .filter(|entity| copied(&entity.id))
        .zip(ids.by_ref())
        .map(|(entity, id)| (entity.id.clone(), EntityId::new(id)))
        .collect();
    let edge_ids: HashMap<EdgeId, EdgeId> = (source.edges())
        .filter(|edge| entity_ids.contains_key(&edge.from) && entity_ids.contains_key(&edge.to))
        .zip(ids)
        .map(|(edge, id)| (edge.id.clone(), EdgeId::new(id)))
        .collect();

    let renamed = |id: &EntityId| entity_ids.get(id).cloned();
    let mut commands = Vec::new();
    // Entities and edges share the stack, so the copies are walked in its
    // order and each goes in front of the one before.
    for item in source.order() {
        let at = at + commands.len();
        match item {
            ItemId::Entity(id) => {
                let (Some(new_id), Some(entity)) = (renamed(id), source.entity(id)) else {
                    continue;
                };
                let (rect, kind) = Start::of(entity).moved(delta);
                let anchor = entity.anchor.clone().and_then(|anchor| {
                    Some(specular_doc::PageAnchor {
                        page_id: renamed(&anchor.page_id)?,
                        ..anchor
                    })
                });
                let parent = entity.parent.as_ref().and_then(|parent| {
                    renamed(parent).or_else(|| match orphan {
                        Orphan::Stays => Some(parent.clone()),
                        Orphan::Leaves => None,
                    })
                });
                let copy = Entity {
                    id: new_id,
                    rect,
                    kind: kind.unwrap_or_else(|| entity.kind.clone()),
                    parent,
                    anchor,
                    ..entity.clone()
                };
                commands.push(Command::InsertEntity {
                    entity: Box::new(copy),
                    at,
                });
            }
            ItemId::Edge(id) => {
                let (Some(new_id), Some(edge)) = (edge_ids.get(id), source.edge(id)) else {
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
    (commands, entity_ids)
}
