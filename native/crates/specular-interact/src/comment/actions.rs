//! What can be done with the comments on the canvas: focus one, resolve it,
//! delete it. Each change to the document is one undo step.

use glam::Vec2;
use serde_json::Value;
use specular_doc::{Annotation, AnnotationId, AnnotationStatus, Command, JsonMap};

use super::marks;
use super::shown::shown;
use crate::{App, Effect, chat, live, update};

/// Gives `id` the focus, and takes the selection away: a comment and a
/// selection are never both what the keys act on. `None` clears the focus,
/// and an id that is not shown does nothing. The comment's thread, when it
/// has a live one, becomes the open thread.
pub(crate) fn focus(app: &mut App, id: Option<&AnnotationId>, effects: &mut Vec<Effect>) {
    let Some(id) = id else {
        app.session.focused_comment = None;
        return;
    };
    if app
        .document
        .annotation(id)
        .is_some_and(|annotation| shown(app, annotation))
    {
        app.session.focused_comment = Some(id.clone());
        app.session.selection.set([]);
        chat::open_thread_of(app, id, effects);
    }
}

/// A left press at `screen`: focuses the comment under it. Returns whether
/// there was one, in which case the press is all it does.
pub(crate) fn press(app: &mut App, screen: Vec2, effects: &mut Vec<Effect>) -> bool {
    let Some(id) = marks::at(app, screen) else {
        return false;
    };
    focus(app, Some(&id), effects);
    true
}

/// Keeps the focus and the selection from both being set, and the focus off
/// a comment that has gone or is no longer shown.
pub(crate) fn settle(app: &mut App) {
    let Some(id) = &app.session.focused_comment else {
        return;
    };
    let alive = (app.document.annotation(id)).is_some_and(|annotation| shown(app, annotation));
    if !alive || !app.session.selection.is_empty() {
        app.session.focused_comment = None;
    }
}

/// The comments an action aimed at `id` acts on: that one, or with `None`
/// every comment on the focused comment's mark.
fn targets(app: &App, id: Option<&AnnotationId>) -> Vec<AnnotationId> {
    if let Some(id) = id {
        return (app.document.annotation(id))
            .map(|_| vec![id.clone()])
            .unwrap_or_default();
    }
    let Some(focused) = &app.session.focused_comment else {
        return Vec::new();
    };
    (app.comment_marks().into_iter())
        .find(|mark| mark.members.contains(focused))
        .map(|mark| mark.members)
        .unwrap_or_default()
}

/// Marks the targets resolved, by the user, as one step. A comment already
/// resolved is left as it is, and when nothing changes no step is made.
pub(crate) fn resolve(app: &mut App, id: Option<&AnnotationId>, effects: &mut Vec<Effect>) {
    let ids = targets(app, id);
    resolve_all(app, &ids, effects);
}

/// Marks `ids` resolved, as [`resolve`] does.
pub(crate) fn resolve_all(app: &mut App, ids: &[AnnotationId], effects: &mut Vec<Effect>) {
    let commands: Vec<Command> = (ids.iter())
        .filter_map(|id| app.document.annotation(id))
        .filter(|annotation| annotation.status != AnnotationStatus::Resolved)
        .map(|annotation| Command::ReplaceAnnotation(Box::new(resolved(annotation))))
        .collect();
    run(app, commands, effects);
}

/// Removes the targets as one step.
pub(crate) fn delete(app: &mut App, id: Option<&AnnotationId>, effects: &mut Vec<Effect>) {
    let commands = targets(app, id)
        .into_iter()
        .map(Command::RemoveAnnotation)
        .collect();
    run(app, commands, effects);
}

/// Records `commands` as one step and lets go of the focus. Does nothing
/// for no commands.
fn run(app: &mut App, commands: Vec<Command>, effects: &mut Vec<Effect>) {
    if commands.is_empty() {
        return;
    }
    update::document_step(app, live::batch(commands), effects);
    app.session.focused_comment = None;
}

/// `annotation` resolved by the user: a resolved thread says who closed it
/// and has no reason for being dismissed.
fn resolved(annotation: &Annotation) -> Annotation {
    let mut metadata: JsonMap = annotation.metadata.clone().unwrap_or_default();
    metadata.remove("dismissReason");
    metadata.insert("resolvedBy".to_owned(), Value::String("user".to_owned()));
    Annotation {
        status: AnnotationStatus::Resolved,
        metadata: Some(metadata),
        ..annotation.clone()
    }
}
