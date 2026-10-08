//! The entities a drag is changing: what they started as, writing each frame
//! into the document, and turning the result into one undo step. And the
//! entity a drag is creating, which sits in the document while it grows.

use glam::DVec2;
use specular_doc::{Command, Document, Drawing, Entity, EntityId, Kind, Rect};

use crate::scroll_follow::Scrolls;
use crate::{App, Effect, anchor, gesture, group_fit, strokes, update};

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

/// The groups above `starts` that a drag refits as it goes, with the rects
/// they had when it began.
pub(crate) fn followers_of(document: &Document, starts: &[Start]) -> Vec<Start> {
    let mut found: Vec<Start> = Vec::new();
    for group in starts
        .iter()
        .flat_map(|start| document.ancestors(&start.id))
    {
        let known = |id: &EntityId| starts.iter().chain(&found).any(|start| start.id == *id);
        if !known(&group.id) {
            found.push(Start::of(group));
        }
    }
    found
}

/// Refits `followers` around what the drag has moved, with no undo step.
pub(crate) fn follow(document: &mut Document, followers: &[Start]) {
    let ids: Vec<EntityId> = followers.iter().map(|start| start.id.clone()).collect();
    group_fit::fit_in_place(document, &ids);
}

/// Turns what a drag left in the document into one undo step whose inverse
/// restores `starts` and `followers`, the groups the drag refitted as it
/// went. `extra` commands join the step; it is given the document as the
/// drag left it, before it is put back. A group among `starts` is moved or
/// resized by the drag itself and is not refitted by the step.
pub(crate) fn commit_following(
    app: &mut App,
    starts: &[Start],
    followers: &[Start],
    extra: impl FnOnce(&App) -> Vec<Command>,
) {
    let mut commands = Vec::new();
    for start in starts.iter().chain(followers) {
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
    commands.extend(extra(app));
    if commands.is_empty() {
        return;
    }
    restore(&mut app.document, starts);
    restore(&mut app.document, followers);
    let moved: Vec<EntityId> = starts.iter().map(|start| start.id.clone()).collect();
    gesture::apply_fitted(app, batch(commands), &moved);
}

/// `commands` as one command.
pub(crate) fn batch(mut commands: Vec<Command>) -> Command {
    if commands.len() == 1 {
        commands.remove(0)
    } else {
        Command::Batch(commands)
    }
}

/// Puts the entity a gesture is creating into the document, in front of
/// everything, replacing the one it put there a frame ago. No undo step.
pub(crate) fn put(document: &mut Document, entity: Entity) {
    take(document, &entity.id);
    let command = Command::InsertEntity {
        entity: Box::new(entity),
        at: document.stack_len(),
    };
    if let Err(error) = document.apply(command) {
        tracing::warn!("creation refused: {error}");
    }
}

/// Takes the entity a gesture was creating back out of the document, with no
/// undo step.
pub(crate) fn take(document: &mut Document, id: &EntityId) -> Option<Entity> {
    let entity = document.entity(id)?.clone();
    match document.apply(Command::RemoveEntity(id.clone())) {
        Ok(_) => Some(entity),
        Err(error) => {
            tracing::warn!("creation not taken back: {error}");
            None
        }
    }
}

/// Adds `entity` in front of everything as one undo step, hooked to the page
/// its centre is on, if any.
pub(crate) fn create(app: &mut App, mut entity: Entity, effects: &mut Vec<Effect>) {
    entity.anchor = anchor::page_anchor_for(&app.document, &Scrolls::of(app), &entity);
    let command = Command::InsertEntity {
        entity: Box::new(entity),
        at: app.document.stack_len(),
    };
    update::document_step(app, command, effects);
}
