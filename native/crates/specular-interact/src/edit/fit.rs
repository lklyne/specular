//! Sizing a text entity to its text, with no undo step.

use specular_doc::{Command, EntityId, Kind, Rect, Text};

use super::{Target, edited, frame, note};
use crate::App;
use crate::saved::LoadedFits;

/// The working text changed. A text entity is resized to fit it, with no
/// undo step: the session's end makes one. A Document is owed a write.
pub(super) fn refit(app: &mut App) {
    let Some(edit) = &app.session.editing else {
        return;
    };
    if edit.target == Target::Note {
        note::touch(app);
        return;
    }
    let Some(entity) = app.document.entity(&edit.entity) else {
        return;
    };
    let rect = match &entity.kind {
        Kind::Text(text) => frame::fitted(entity.rect, text, &edit.text, app.measure.0.as_ref()),
        Kind::Shape(_) | Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => {
            return;
        }
    };
    let id = edit.entity.clone();
    set_rect(app, &id, rect);
}

/// Refits the text being edited after something other than typing changed
/// how it sets, such as its size or typeface.
pub(crate) fn refit_edited(app: &mut App) {
    if edited(app).is_some() {
        refit(app);
    }
}

/// The size `text` takes at `rect` to fit its own text: what a resize and a
/// load give a text entity, so its height is its content's.
pub(crate) fn fitted(app: &App, rect: Rect, text: &Text) -> Rect {
    frame::fitted(rect, text, &text.text, app.measure.0.as_ref())
}

/// Gives every text entity the size its text takes, with no undo step. A
/// document from disk carries heights measured with another renderer's
/// fonts. Nothing happens unless the measure is the renderer's own. What
/// each text was read with is kept for the save (see `saved.rs`).
pub(crate) fn fit_all(app: &mut App) {
    app.session.loaded_fits = LoadedFits::default();
    if !app.measure.0.is_exact() {
        return;
    }
    let fits: Vec<(EntityId, Rect, Rect)> = (app.document.entities())
        .filter_map(|entity| match &entity.kind {
            Kind::Text(text) => {
                let fitted = fitted(app, entity.rect, text);
                Some((entity.id.clone(), entity.rect, fitted))
            }
            Kind::Shape(_) | Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => {
                None
            }
        })
        .collect();
    for (id, read, fitted) in fits {
        set_rect(app, &id, fitted);
        app.session.loaded_fits.insert(id, read, fitted);
    }
}

/// Writes `rect` into the document with no undo step.
pub(super) fn set_rect(app: &mut App, id: &EntityId, rect: Rect) {
    if app
        .document
        .entity(id)
        .is_none_or(|entity| entity.rect == rect)
    {
        return;
    }
    let command = Command::SetRect {
        id: id.clone(),
        rect,
    };
    if let Err(error) = app.document.apply(command) {
        tracing::warn!("text resize refused: {error}");
    }
}
