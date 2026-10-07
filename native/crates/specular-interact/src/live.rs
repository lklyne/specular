//! The entities a drag is changing: what they started as, writing each frame
//! into the document, and turning the result into one undo step.

use glam::DVec2;
use specular_doc::{Command, Document, Drawing, Entity, EntityId, Kind, Rect};

use crate::{App, gesture, strokes};

/// An entity as a drag found it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Start {
    pub(crate) id: EntityId,
    pub(crate) rect: Rect,
    /// The kind fields, for the kinds a drag can change them on.
    pub(crate) kind: Option<Kind>,
}

impl Start {
    pub(crate) fn of(entity: &Entity) -> Self {
        let kind = match &entity.kind {
            // A drawing's points move with it, and a text's size follows a
            // resize.
            Kind::Drawing(_) | Kind::Text(_) => Some(entity.kind.clone()),
            Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Shape(_) => None,
        };
        Self {
            id: entity.id.clone(),
            rect: entity.rect,
            kind,
        }
    }

    /// The rect and kind this entity has `delta` away from where it started.
    pub(crate) fn moved(&self, delta: DVec2) -> (Rect, Option<Kind>) {
        let kind = match &self.kind {
            Some(Kind::Drawing(drawing)) => Some(Kind::Drawing(Drawing {
                strokes: strokes::translated(&drawing.strokes, delta),
            })),
            Some(
                kind @ (Kind::Page(_)
                | Kind::Text(_)
                | Kind::File(_)
                | Kind::Group(_)
                | Kind::Shape(_)),
            ) => Some(kind.clone()),
            None => None,
        };
        (self.rect.translated(delta.x, delta.y), kind)
    }
}

/// The starts of `ids`, skipping ids that name nothing.
pub(crate) fn starts(document: &Document, ids: &[EntityId]) -> Vec<Start> {
    ids.iter()
        .filter_map(|id| document.entity(id))
        .map(Start::of)
        .collect()
}

/// The commands that give `start`'s entity this rect and kind, leaving out
/// whatever it already has.
fn changes(document: &Document, start: &Start, rect: Rect, kind: Option<Kind>) -> Vec<Command> {
    let Some(entity) = document.entity(&start.id) else {
        return Vec::new();
    };
    let mut commands = Vec::new();
    if entity.rect != rect {
        commands.push(Command::SetRect {
            id: start.id.clone(),
            rect,
        });
    }
    if let Some(kind) = kind
        && entity.kind != kind
    {
        commands.push(Command::SetKind {
            id: start.id.clone(),
            kind: Box::new(kind),
        });
    }
    commands
}

/// Writes one frame of a drag, with no undo step.
pub(crate) fn write(document: &mut Document, start: &Start, rect: Rect, kind: Option<Kind>) {
    for command in changes(document, start, rect, kind) {
        if let Err(error) = document.apply(command) {
            tracing::warn!("drag refused: {error}");
        }
    }
}

/// Puts every entity back as the drag found it, with no undo step.
pub(crate) fn restore(document: &mut Document, starts: &[Start]) {
    for start in starts {
        write(document, start, start.rect, start.kind.clone());
    }
}

/// Turns what a drag left in the document into one undo step whose inverse
/// restores `starts`.
pub(crate) fn commit(app: &mut App, starts: &[Start]) {
    let mut commands = Vec::new();
    for start in starts {
        let Some(entity) = app.document.entity(&start.id) else {
            continue;
        };
        if entity.rect != start.rect {
            commands.push(Command::SetRect {
                id: start.id.clone(),
                rect: entity.rect,
            });
        }
        if start.kind.as_ref().is_some_and(|kind| *kind != entity.kind) {
            commands.push(Command::SetKind {
                id: start.id.clone(),
                kind: Box::new(entity.kind.clone()),
            });
        }
    }
    if commands.is_empty() {
        return;
    }
    restore(&mut app.document, starts);
    gesture::apply_step(app, batch(commands));
}

/// `commands` as one command.
pub(crate) fn batch(mut commands: Vec<Command>) -> Command {
    if commands.len() == 1 {
        commands.remove(0)
    } else {
        Command::Batch(commands)
    }
}
