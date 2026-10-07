//! In-place text editing: a text or sticky entity's text, or a shape's
//! label.
//!
//! [`Session::editing`](crate::Session::editing) holds the [`TextEdit`]: the
//! working text, the caret and the selection. The document keeps the old
//! text until the session ends, which is one undo step however much was
//! typed. Inside the session Command+Z steps the editor's own undo.
//!
//! The entity's rect is the exception: a text entity grows with its text,
//! and that is written into the document as it happens, as a drag writes
//! rects, so the outline, the handles and hit-testing follow. Ending the
//! session puts the old rect back and makes the step.
//!
//! Nothing here knows about fonts. Whatever depends on where glyphs land
//! goes through the [`TextMeasure`] the [`App`] holds.

mod blink;
mod buffer;
mod frame;
mod history;
mod ime;
mod keys;
mod layout;
mod lists;
mod measure;
mod motion;
mod pointer;
mod segment;

use std::sync::Arc;

use glam::Vec2;
use specular_core::ImeEvent;
use specular_doc::{Command, Entity, EntityId, ItemId, Kind, Rect, Text};

pub(crate) use blink::{caret_state, restart_blink};
pub use buffer::TextEdit;
use buffer::{Origin, Target};
pub use frame::TextFrame;
use history::Change;
pub(crate) use keys::on_key;
pub(crate) use measure::Measurer;
pub use measure::{CaretStop, LayoutLine, TextLayout, TextMeasure, TextSpec};
pub use pointer::TextSelectDrag;
pub(crate) use pointer::{drag, is_over_text, press};

use crate::{App, Effect, live, update};

/// The text of `entity` that can be edited in place, and what it is.
fn editable(entity: &Entity) -> Option<(Target, &str)> {
    match &entity.kind {
        Kind::Text(text) => Some((Target::Text, &text.text)),
        Kind::Shape(shape) => Some((Target::Label, &shape.text)),
        Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => None,
    }
}

/// `kind` with `text` as its text.
fn with_text(kind: &Kind, text: String) -> Kind {
    let mut kind = kind.clone();
    match &mut kind {
        Kind::Text(fields) => fields.text = text,
        Kind::Shape(fields) => fields.text = text,
        Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => {}
    }
    kind
}

/// Where `edit`'s text sits and how its lines fall, as it stands now.
fn geometry(app: &App, edit: &TextEdit) -> Option<(TextFrame, TextLayout)> {
    let frame = frame::of(app.document.entity(&edit.entity)?)?;
    let layout = app.measure.0.layout(&edit.text, &frame.spec);
    Some((frame, layout))
}

fn layout_of(app: &App, edit: &TextEdit) -> Option<TextLayout> {
    geometry(app, edit).map(|(_, layout)| layout)
}

/// Starts editing `id`'s text with all of it selected, if it has text to
/// edit, and makes it the selection. `created` says the entity was put in
/// the document for this session and has no undo step yet.
pub(crate) fn begin(app: &mut App, id: &EntityId, created: bool, effects: &mut Vec<Effect>) {
    end(app, effects);
    let Some(entity) = app.document.entity(id) else {
        return;
    };
    let Some((target, text)) = editable(entity) else {
        return;
    };
    let origin = Origin {
        text: text.to_owned(),
        rect: entity.rect,
        created,
    };
    let mut edit = TextEdit::new(id.clone(), target, text, origin);
    edit.active_ms = app.session.now_ms;
    app.session.editing = Some(edit);
    app.session.selection.set([ItemId::Entity(id.clone())]);
    effects.push(Effect::SetImeAllowed(true));
    if created {
        refit(app);
    }
    place_candidates(app, effects);
}

/// Resizes the text entity being edited to fit its working text. No undo
/// step: the session's end makes one.
fn refit(app: &mut App) {
    let Some(edit) = &app.session.editing else {
        return;
    };
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

/// The size `text` takes at `rect` to fit its own text: what a resize and a
/// load give a text entity, so its height is its content's.
pub(crate) fn fitted(app: &App, rect: Rect, text: &Text) -> Rect {
    frame::fitted(rect, text, &text.text, app.measure.0.as_ref())
}

/// Gives every text entity the size its text takes, with no undo step. A
/// document from disk carries heights measured with another renderer's
/// fonts. Nothing happens unless the measure is the renderer's own.
pub(crate) fn fit_all(app: &mut App) {
    if !app.measure.0.is_exact() {
        return;
    }
    let fits: Vec<(EntityId, Rect)> = (app.document.entities())
        .filter_map(|entity| match &entity.kind {
            Kind::Text(text) => Some((entity.id.clone(), fitted(app, entity.rect, text))),
            Kind::Shape(_) | Kind::Page(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => {
                None
            }
        })
        .collect();
    for (id, rect) in fits {
        set_rect(app, &id, rect);
    }
}

/// Writes `rect` into the document with no undo step.
fn set_rect(app: &mut App, id: &EntityId, rect: Rect) {
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

/// Ends the edit session, if there is one, and turns it into at most one
/// undo step: the new text with the rect that fits it. A text entity left
/// with nothing in it is deleted, and one that was placed for this session
/// then leaves no step at all.
pub(crate) fn end(app: &mut App, effects: &mut Vec<Effect>) {
    let Some(edit) = app.session.editing.take() else {
        return;
    };
    effects.push(Effect::SetImeAllowed(false));
    let id = edit.entity.clone();
    let Some(entity) = app.document.entity(&id).cloned() else {
        return;
    };
    let emptied = edit.target == Target::Text && edit.text.trim().is_empty();
    let kind = with_text(&entity.kind, edit.text);
    if edit.origin.created {
        live::take(&mut app.document, &id);
        if emptied {
            forget(app, &id);
        } else {
            live::create(app, Entity { kind, ..entity }, effects);
        }
        return;
    }
    set_rect(app, &id, edit.origin.rect);
    if kind == entity.kind && !emptied {
        return;
    }
    let command = if emptied {
        let edges = app.document.edges_touching(&id);
        let mut commands: Vec<Command> = edges
            .map(|edge| Command::RemoveEdge(edge.id.clone()))
            .collect();
        commands.push(Command::RemoveEntity(id));
        live::batch(commands)
    } else {
        let mut commands = vec![Command::SetKind {
            id: id.clone(),
            kind: Box::new(kind),
        }];
        if entity.rect != edit.origin.rect {
            commands.push(Command::SetRect {
                id,
                rect: entity.rect,
            });
        }
        live::batch(commands)
    };
    update::document_step(app, command, effects);
}

/// Drops the edit session without keeping any of it: the document it was
/// editing has been replaced.
pub(crate) fn discard(app: &mut App, effects: &mut Vec<Effect>) {
    if app.session.editing.take().is_some() {
        effects.push(Effect::SetImeAllowed(false));
    }
}

/// Takes an entity that left the document with no undo step out of the
/// selection and the hover.
fn forget(app: &mut App, id: &EntityId) {
    let gone = ItemId::Entity(id.clone());
    app.session.selection.retain(|item| *item != gone);
    if app.session.hover.as_ref() == Some(id) {
        app.session.hover = None;
    }
}

/// Steps the editor's own undo back or forward.
pub(crate) fn step_history(app: &mut App, back: bool) {
    let Some(edit) = &mut app.session.editing else {
        return;
    };
    if edit.composition.is_some() {
        return;
    }
    if if back { edit.undo() } else { edit.redo() } {
        refit(app);
    }
}

/// Text from the clipboard, typed over the selection as one step.
pub(crate) fn paste(app: &mut App, text: &str) {
    let Some(edit) = &mut app.session.editing else {
        return;
    };
    edit.composition = None;
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    if edit.insert(&text, Change::Single) {
        refit(app);
    }
}

/// An input-method event for the text being edited.
pub(crate) fn on_ime(app: &mut App, event: &ImeEvent, effects: &mut Vec<Effect>) {
    let Some(edit) = &mut app.session.editing else {
        return;
    };
    if ime::apply(edit, event) {
        refit(app);
    }
    place_candidates(app, effects);
}

/// Tells the shell where the caret is on screen, so the input method's
/// candidate window opens beside it.
fn place_candidates(app: &App, effects: &mut Vec<Effect>) {
    if let Some(caret) = app.caret_rect() {
        let camera = &app.session.camera;
        let corner = Vec2::new(caret.x as f32, caret.y as f32);
        effects.push(Effect::SetImeCursorArea {
            origin: camera.world_to_screen(corner),
            size: Vec2::new(caret.width as f32, caret.height as f32) * camera.zoom,
        });
    }
}

/// What the scene draws for the text being edited. Rects are in canvas
/// space.
impl App {
    /// Installs the measure that lays text out for the editor. The shell
    /// gives one that shapes with the renderer's fonts.
    pub fn set_text_measure(&mut self, measure: Arc<dyn TextMeasure>) {
        self.measure = Measurer(measure);
    }

    /// The measure the editor lays text out with.
    pub fn text_measure(&self) -> &dyn TextMeasure {
        self.measure.0.as_ref()
    }

    /// The edit session, if text is being edited.
    pub fn text_edit(&self) -> Option<&TextEdit> {
        self.session.editing.as_ref()
    }

    /// The text to draw for `id` in place of the document's, when `id` is
    /// the entity being edited.
    pub fn editing_text(&self, id: &EntityId) -> Option<&str> {
        let edit = self.session.editing.as_ref()?;
        (edit.entity == *id).then_some(edit.text.as_str())
    }

    /// Where `id`'s text is laid out and how it is set, for a text, a
    /// sticky or a shape.
    pub fn text_frame(&self, id: &EntityId) -> Option<TextFrame> {
        frame::of(self.document.entity(id)?)
    }

    /// The caret: a rect with no width, one line tall. It is there whether
    /// or not anything is selected.
    pub fn caret_rect(&self) -> Option<Rect> {
        let edit = self.session.editing.as_ref()?;
        let (frame, layout) = geometry(self, edit)?;
        let caret = layout.caret_box(edit.caret)?;
        Some(frame.rect_of(&layout, caret))
    }

    /// The selected text, one rect per line it touches. Empty when nothing
    /// is selected.
    pub fn selection_rects(&self) -> Vec<Rect> {
        self.range_rects(TextEdit::selection)
    }

    /// The text the input method is composing, one rect per line it
    /// touches, to underline.
    pub fn composition_rects(&self) -> Vec<Rect> {
        self.range_rects(|edit| edit.composition().unwrap_or_default())
    }

    fn range_rects(&self, range: impl FnOnce(&TextEdit) -> std::ops::Range<usize>) -> Vec<Rect> {
        let Some(edit) = &self.session.editing else {
            return Vec::new();
        };
        let Some((frame, layout)) = geometry(self, edit) else {
            return Vec::new();
        };
        let boxes = layout.range_boxes(&range(edit));
        (boxes.into_iter())
            .map(|line| frame.rect_of(&layout, line))
            .collect()
    }
}
