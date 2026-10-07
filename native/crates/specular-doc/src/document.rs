//! [`Document`]: everything a `.canvas` file holds.

mod apply;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::{Annotation, AnnotationId, Edge, EdgeId, Entity, EntityId, ItemId, JsonMap, Kind};

/// A canvas: entities and edges in one stack order, plus annotations.
///
/// Reads are plain accessors. Writes go through
/// [`apply`](Self::apply), except [`extra_mut`](Self::extra_mut), which holds
/// view state that is not undoable.
///
/// References between items are ids and may dangle: a file written by
/// another tool can name a missing group or page, and loading must keep it.
/// Accessors that follow a reference return `None` or skip when it dangles.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Document {
    entities: HashMap<EntityId, Entity>,
    edges: HashMap<EdgeId, Edge>,
    order: Vec<ItemId>,
    annotations: Vec<Annotation>,
    extra: JsonMap,
}

impl Document {
    /// An empty document.
    pub fn new() -> Self {
        Self::default()
    }

    /// The entity with this id.
    pub fn entity(&self, id: &EntityId) -> Option<&Entity> {
        self.entities.get(id)
    }

    /// The edge with this id.
    pub fn edge(&self, id: &EdgeId) -> Option<&Edge> {
        self.edges.get(id)
    }

    /// The annotation with this id.
    pub fn annotation(&self, id: &AnnotationId) -> Option<&Annotation> {
        self.annotations
            .iter()
            .find(|annotation| annotation.id == *id)
    }

    /// The stack order, back-to-front. Every entity and edge has one slot.
    pub fn order(&self) -> &[ItemId] {
        &self.order
    }

    /// Number of slots in the stack order: the index that inserts in front.
    pub fn stack_len(&self) -> usize {
        self.order.len()
    }

    /// An item's index in the stack order.
    pub fn stack_index(&self, item: &ItemId) -> Option<usize> {
        self.order.iter().position(|slot| slot == item)
    }

    /// Entities, back-to-front.
    pub fn entities(&self) -> impl DoubleEndedIterator<Item = &Entity> {
        self.order.iter().filter_map(|slot| match slot {
            ItemId::Entity(id) => self.entities.get(id),
            ItemId::Edge(_) => None,
        })
    }

    /// Edges, back-to-front.
    pub fn edges(&self) -> impl DoubleEndedIterator<Item = &Edge> {
        self.order.iter().filter_map(|slot| match slot {
            ItemId::Edge(id) => self.edges.get(id),
            ItemId::Entity(_) => None,
        })
    }

    /// Annotations, in file order.
    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    /// The entities whose `parent` is `group`, back-to-front.
    pub fn children(&self, group: &EntityId) -> impl Iterator<Item = &Entity> {
        self.entities()
            .filter(move |entity| entity.parent.as_ref() == Some(group))
    }

    /// The groups containing `id`, innermost first. Stops at a missing
    /// parent, and at a cycle a hand-edited file could contain.
    pub fn ancestors(&self, id: &EntityId) -> impl Iterator<Item = &Entity> {
        let mut next = self
            .entities
            .get(id)
            .and_then(|entity| entity.parent.as_ref());
        let mut remaining = self.entities.len();
        std::iter::from_fn(move || {
            remaining = remaining.checked_sub(1)?;
            let group = self.entities.get(next?)?;
            next = group.parent.as_ref();
            Some(group)
        })
    }

    /// The edges that start or end at `id`, back-to-front.
    pub fn edges_touching(&self, id: &EntityId) -> impl Iterator<Item = &Edge> {
        self.edges().filter(move |edge| edge.touches(id))
    }

    /// Unmodeled top-level fields of the file, such as `appState`.
    pub fn extra(&self) -> &JsonMap {
        &self.extra
    }

    /// Mutable access to the unmodeled top-level fields. These are outside
    /// the command system, so changing them is not an undo step.
    pub fn extra_mut(&mut self) -> &mut JsonMap {
        &mut self.extra
    }

    /// Whether an entity or edge already uses this id.
    fn stack_id_taken(&self, id: &str) -> bool {
        self.order.iter().any(|slot| slot.as_str() == id)
    }

    fn is_group(&self, id: &EntityId) -> bool {
        matches!(
            self.entities.get(id),
            Some(Entity {
                kind: Kind::Group(_),
                ..
            })
        )
    }
}
