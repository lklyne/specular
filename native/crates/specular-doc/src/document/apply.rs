//! [`Document::apply`]: run a command, return its inverse.

use std::collections::HashSet;
use std::mem;

use super::Document;
use crate::{
    Annotation, AnnotationId, Command, CommandError, Edge, EdgeId, Entity, EntityId, ItemId, Kind,
};

impl Document {
    /// Runs `command` and returns the command that undoes it. Applying the
    /// returned command restores the document exactly and returns a command
    /// equivalent to the original.
    ///
    /// On error the document is unchanged.
    pub fn apply(&mut self, command: Command) -> Result<Command, CommandError> {
        match command {
            Command::InsertEntity { entity, at } => self.insert_entity(*entity, at),
            Command::RemoveEntity(id) => self.remove_entity(id),
            Command::SetRect { id, rect } => {
                let rect = mem::replace(&mut self.entity_mut(&id)?.rect, rect);
                Ok(Command::SetRect { id, rect })
            }
            Command::SetLabel { id, label } => {
                let label = mem::replace(&mut self.entity_mut(&id)?.label, label);
                Ok(Command::SetLabel { id, label })
            }
            Command::SetParent { id, parent } => {
                if let Some(parent) = &parent {
                    self.check_parent(&id, parent)?;
                }
                let parent = mem::replace(&mut self.entity_mut(&id)?.parent, parent);
                Ok(Command::SetParent { id, parent })
            }
            Command::SetAnchor { id, anchor } => {
                let anchor = anchor.map(|anchor| *anchor);
                let anchor = mem::replace(&mut self.entity_mut(&id)?.anchor, anchor);
                Ok(Command::SetAnchor {
                    id,
                    anchor: anchor.map(Box::new),
                })
            }
            Command::SetKind { id, kind } => self.set_kind(id, kind),
            Command::InsertEdge { edge, at } => self.insert_edge(*edge, at),
            Command::RemoveEdge(id) => self.remove_edge(id),
            Command::ReplaceEdge(edge) => self.replace_edge(edge),
            Command::SetOrder(order) => self.set_order(order),
            Command::InsertAnnotation { annotation, at } => self.insert_annotation(*annotation, at),
            Command::RemoveAnnotation(id) => self.remove_annotation(id),
            Command::ReplaceAnnotation(annotation) => self.replace_annotation(annotation),
            Command::Batch(commands) => self.apply_batch(commands),
        }
    }

    fn insert_entity(&mut self, entity: Entity, at: usize) -> Result<Command, CommandError> {
        self.check_insert(entity.id.as_str(), at)?;
        let id = entity.id.clone();
        self.order.insert(at, ItemId::Entity(id.clone()));
        self.entities.insert(id.clone(), entity);
        Ok(Command::RemoveEntity(id))
    }

    fn remove_entity(&mut self, id: EntityId) -> Result<Command, CommandError> {
        let Some(entity) = self.entities.remove(&id) else {
            return Err(CommandError::UnknownEntity(id));
        };
        Ok(Command::InsertEntity {
            entity: Box::new(entity),
            at: self.take_slot(&ItemId::Entity(id)),
        })
    }

    fn set_kind(&mut self, id: EntityId, mut kind: Box<Kind>) -> Result<Command, CommandError> {
        let entity = self.entity_mut(&id)?;
        if mem::discriminant(&entity.kind) != mem::discriminant(&*kind) {
            return Err(CommandError::KindMismatch {
                expected: entity.kind.name(),
                found: kind.name(),
                id,
            });
        }
        mem::swap(&mut entity.kind, &mut *kind);
        Ok(Command::SetKind { id, kind })
    }

    fn insert_edge(&mut self, edge: Edge, at: usize) -> Result<Command, CommandError> {
        self.check_insert(edge.id.as_str(), at)?;
        let id = edge.id.clone();
        self.order.insert(at, ItemId::Edge(id.clone()));
        self.edges.insert(id.clone(), edge);
        Ok(Command::RemoveEdge(id))
    }

    fn remove_edge(&mut self, id: EdgeId) -> Result<Command, CommandError> {
        let Some(edge) = self.edges.remove(&id) else {
            return Err(CommandError::UnknownEdge(id));
        };
        Ok(Command::InsertEdge {
            edge: Box::new(edge),
            at: self.take_slot(&ItemId::Edge(id)),
        })
    }

    fn replace_edge(&mut self, mut edge: Box<Edge>) -> Result<Command, CommandError> {
        let Some(current) = self.edges.get_mut(&edge.id) else {
            return Err(CommandError::UnknownEdge(edge.id));
        };
        mem::swap(current, &mut *edge);
        Ok(Command::ReplaceEdge(edge))
    }

    fn set_order(&mut self, order: Vec<ItemId>) -> Result<Command, CommandError> {
        let current: HashSet<&ItemId> = self.order.iter().collect();
        let proposed: HashSet<&ItemId> = order.iter().collect();
        if order.len() != self.order.len() || proposed != current {
            return Err(CommandError::OrderMismatch);
        }
        Ok(Command::SetOrder(mem::replace(&mut self.order, order)))
    }

    fn insert_annotation(
        &mut self,
        annotation: Annotation,
        at: usize,
    ) -> Result<Command, CommandError> {
        if self.annotation(&annotation.id).is_some() {
            return Err(CommandError::DuplicateId(annotation.id.as_str().to_owned()));
        }
        check_index(at, self.annotations.len())?;
        let id = annotation.id.clone();
        self.annotations.insert(at, annotation);
        Ok(Command::RemoveAnnotation(id))
    }

    fn remove_annotation(&mut self, id: AnnotationId) -> Result<Command, CommandError> {
        let Some(at) = self.annotations.iter().position(|a| a.id == id) else {
            return Err(CommandError::UnknownAnnotation(id));
        };
        Ok(Command::InsertAnnotation {
            annotation: Box::new(self.annotations.remove(at)),
            at,
        })
    }

    fn replace_annotation(
        &mut self,
        mut annotation: Box<Annotation>,
    ) -> Result<Command, CommandError> {
        let Some(current) = self
            .annotations
            .iter_mut()
            .find(|current| current.id == annotation.id)
        else {
            return Err(CommandError::UnknownAnnotation(annotation.id));
        };
        mem::swap(current, &mut *annotation);
        Ok(Command::ReplaceAnnotation(annotation))
    }

    fn apply_batch(&mut self, commands: Vec<Command>) -> Result<Command, CommandError> {
        let mut inverses = Vec::with_capacity(commands.len());
        for command in commands {
            match self.apply(command) {
                Ok(inverse) => inverses.push(inverse),
                Err(error) => {
                    for inverse in inverses.into_iter().rev() {
                        let restored = self.apply(inverse);
                        debug_assert!(restored.is_ok(), "an inverse must apply: {restored:?}");
                    }
                    return Err(error);
                }
            }
        }
        inverses.reverse();
        Ok(Command::Batch(inverses))
    }

    fn entity_mut(&mut self, id: &EntityId) -> Result<&mut Entity, CommandError> {
        self.entities
            .get_mut(id)
            .ok_or_else(|| CommandError::UnknownEntity(id.clone()))
    }

    fn check_insert(&self, id: &str, at: usize) -> Result<(), CommandError> {
        if self.stack_id_taken(id) {
            return Err(CommandError::DuplicateId(id.to_owned()));
        }
        check_index(at, self.order.len())
    }

    fn check_parent(&self, id: &EntityId, parent: &EntityId) -> Result<(), CommandError> {
        let inside_itself = parent == id || self.ancestors(parent).any(|group| group.id == *id);
        if inside_itself || !self.is_group(parent) {
            return Err(CommandError::InvalidParent {
                id: id.clone(),
                parent: parent.clone(),
            });
        }
        Ok(())
    }

    /// Removes an item's stack slot and returns the index it held.
    fn take_slot(&mut self, item: &ItemId) -> usize {
        let at = self.stack_index(item);
        debug_assert!(at.is_some(), "every entity and edge has a stack slot");
        at.map_or(self.order.len(), |at| {
            self.order.remove(at);
            at
        })
    }
}

fn check_index(index: usize, len: usize) -> Result<(), CommandError> {
    if index > len {
        return Err(CommandError::IndexOutOfRange { index, len });
    }
    Ok(())
}
