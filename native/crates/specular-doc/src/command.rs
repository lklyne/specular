//! [`Command`]: the only way a [`Document`](crate::Document) changes.

use crate::{
    Annotation, AnnotationId, Edge, EdgeId, Entity, EntityId, ItemId, Kind, PageAnchor, Rect,
};

/// One document mutation. [`Document::apply`](crate::Document::apply) runs
/// it and returns the command that undoes it.
///
/// Commands are primitive: none of them cascades. Removing a group leaves
/// its members' `parent` pointing at it, and removing an entity leaves its
/// edges in place. A caller that wants a cascade builds a [`Command::Batch`]
/// from [`Document::children`](crate::Document::children) and
/// [`Document::edges_touching`](crate::Document::edges_touching).
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Adds an entity at stack index `at` (0 is the back,
    /// [`Document::stack_len`](crate::Document::stack_len) the front).
    InsertEntity {
        /// The entity to add.
        entity: Box<Entity>,
        /// Its stack index.
        at: usize,
    },
    /// Removes an entity.
    RemoveEntity(EntityId),
    /// Moves or resizes an entity.
    SetRect {
        /// The entity.
        id: EntityId,
        /// Its new rect.
        rect: Rect,
    },
    /// Renames an entity.
    SetLabel {
        /// The entity.
        id: EntityId,
        /// Its new label.
        label: Option<String>,
    },
    /// Moves an entity into a group, or out of any group. The parent must be
    /// an existing group that is not the entity or one of its descendants.
    SetParent {
        /// The entity.
        id: EntityId,
        /// Its new group.
        parent: Option<EntityId>,
    },
    /// Hooks an entity to a page, or frees it.
    SetAnchor {
        /// The entity.
        id: EntityId,
        /// Its new anchor.
        anchor: Option<Box<PageAnchor>>,
    },
    /// Replaces an entity's kind fields. The variant must not change.
    SetKind {
        /// The entity.
        id: EntityId,
        /// Its new fields.
        kind: Box<Kind>,
    },
    /// Adds an edge at stack index `at`.
    InsertEdge {
        /// The edge to add.
        edge: Box<Edge>,
        /// Its stack index.
        at: usize,
    },
    /// Removes an edge.
    RemoveEdge(EdgeId),
    /// Replaces the edge that has this edge's id.
    ReplaceEdge(Box<Edge>),
    /// Replaces the stack order. The new order must hold exactly the ids the
    /// current one does.
    SetOrder(Vec<ItemId>),
    /// Adds an annotation at index `at` of the annotation list.
    InsertAnnotation {
        /// The annotation to add.
        annotation: Box<Annotation>,
        /// Its index.
        at: usize,
    },
    /// Removes an annotation.
    RemoveAnnotation(AnnotationId),
    /// Replaces the annotation that has this annotation's id.
    ReplaceAnnotation(Box<Annotation>),
    /// Runs the commands in order as one step. If one fails, the ones before
    /// it are undone and the document is left as it was.
    Batch(Vec<Command>),
}

/// Why a [`Command`] was refused. A refused command leaves the document
/// unchanged.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CommandError {
    /// No entity has this id.
    #[error("unknown entity {0:?}")]
    UnknownEntity(EntityId),
    /// No edge has this id.
    #[error("unknown edge {0:?}")]
    UnknownEdge(EdgeId),
    /// No annotation has this id.
    #[error("unknown annotation {0:?}")]
    UnknownAnnotation(AnnotationId),
    /// An entity, edge or annotation with this id already exists.
    #[error("duplicate id {0:?}")]
    DuplicateId(String),
    /// An insert index past the end of its list.
    #[error("index {index} is out of range for a list of {len}")]
    IndexOutOfRange {
        /// The requested index.
        index: usize,
        /// The list's length.
        len: usize,
    },
    /// `SetKind` tried to turn one kind into another.
    #[error("entity {id:?} is a {expected}, not a {found}")]
    KindMismatch {
        /// The entity.
        id: EntityId,
        /// The kind it is.
        expected: &'static str,
        /// The kind the command carried.
        found: &'static str,
    },
    /// `SetParent` named a parent that is missing, is not a group, or sits
    /// inside the entity.
    #[error("{parent:?} cannot be the parent of {id:?}")]
    InvalidParent {
        /// The entity.
        id: EntityId,
        /// The refused parent.
        parent: EntityId,
    },
    /// `SetOrder` carried ids that differ from the current stack's.
    #[error("the new stack order does not hold the same ids as the current one")]
    OrderMismatch,
}
