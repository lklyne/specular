//! In-place text editing: a text or sticky entity's text, a shape's label,
//! the markdown source of a Document, a title, or a comment draft.
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
//! A Document's text is a file's, not the document's. It is edited the same
//! way; [`note`] has what differs: writing the file as the text changes, and
//! what ending the session records.
//!
//! Nothing here knows about fonts. Whatever depends on where glyphs land
//! goes through the [`TextMeasure`] the [`App`] holds.

mod blink;
mod buffer;
mod edge_label;
mod field;
mod fit;
mod format;
mod formatting;
pub(crate) mod frame;
mod history;
mod ime;
mod keys;
mod lists;
mod measure;
mod motion;
pub(crate) mod note;
mod pointer;
mod read;
mod segment;
mod source;
mod stack;
mod title;

use std::sync::Arc;

use glam::Vec2;
use specular_core::ImeEvent;
use specular_doc::{Command, Entity, EntityId, ItemId, Kind};

pub(crate) use blink::{caret_state, restart_blink};
pub use buffer::TextEdit;
pub(crate) use buffer::{Origin, Target};
pub use edge_label::LABEL_SIZE as EDGE_LABEL_SIZE;
pub(crate) use edge_label::begin as begin_edge_label;
pub(crate) use edge_label::selected_key as selected_edge_key;
pub(crate) use field::{
    begin as begin_field, cancel as cancel_field, focus as focus_field,
    follow_caret as follow_field_caret,
};
pub(crate) use fit::{fit_all, fitted, refit_edited};
use fit::{refit, set_rect};
pub use formatting::Format;
pub(crate) use formatting::run as format;
pub use frame::{NOTE_PADDING, TextFrame, note_frame};
use history::Change;
pub(crate) use measure::Measurer;
pub use measure::{CaretStop, LayoutLine, TextLayout, TextMeasure, TextSpec};
pub use pointer::TextSelectDrag;
pub(crate) use pointer::{autoscroll, drag, is_over_text, press};
pub use read::EditMarks;
pub use source::{SourceLine, SourceSpan, SourceStyle, style_lines};
pub use stack::{SourceRow, StackCache, source_rows};
pub use title::{TITLE_GAP, TITLE_LINE, TITLE_SIZE};
pub(crate) use title::{is_editing as is_editing_title, on_key};

use crate::{App, Effect, comment, live, update};

/// The text of `entity` that can be edited in place, and what it is. A
/// Document can be edited once its file has been read.
fn editable<'a>(app: &'a App, entity: &'a Entity) -> Option<(Target, &'a str)> {
    match &entity.kind {
        Kind::Text(text) => Some((Target::Text, &text.text)),
        Kind::Shape(shape) => Some((Target::Label, &shape.text)),
        Kind::File(_) => {
            let file = note::file_of(entity)?;
            Some((Target::Note, note::text_of(app.session.notes.get(file))?))
        }
        Kind::Group(_) => Some((Target::Title, entity.label.as_deref().unwrap_or_default())),
        Kind::Page(_) | Kind::Drawing(_) => None,
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

/// Where `edit`'s text sits and how it is set. Nothing is measured.
fn frame_of(app: &App, edit: &TextEdit) -> Option<TextFrame> {
    match edit.target {
        Target::EdgeLabel => edge_label::frame(app, &edit.entity),
        Target::Comment => comment::frame(app),
        Target::Field => field::frame(app, edit),
        Target::Text | Target::Label => {
            let entity = app.document.entity(&edit.entity)?;
            frame::of(entity, crate::scroll_follow::placed_rect(app, entity))
        }
        Target::Title => title::frame(app, app.document.entity(&edit.entity)?),
        Target::Note => {
            let entity = app.document.entity(&edit.entity)?;
            let scroll = app.session.notes.scroll(&entity.id);
            let rect = crate::scroll_follow::placed_rect(app, entity);
            Some(frame::note_frame(rect, scroll))
        }
    }
}

/// Where `edit`'s text sits and how its lines fall, as it stands now. A
/// Document's rows are looked up in `stacks` before they are measured.
fn geometry_in(
    app: &App,
    edit: &TextEdit,
    stacks: &StackCache,
) -> Option<(TextFrame, Arc<TextLayout>)> {
    let frame = frame_of(app, edit)?;
    let layout = match edit.target {
        Target::Note => stack::layout(&edit.text, &frame.spec, &app.measure.0, stacks),
        Target::Text
        | Target::Label
        | Target::Title
        | Target::EdgeLabel
        | Target::Comment
        | Target::Field => Arc::new(app.measure.0.layout(&edit.text, &frame.spec)),
    };
    Some((frame, layout))
}

/// [`geometry_in`] with nothing kept, which is how `update` asks.
fn geometry(app: &App, edit: &TextEdit) -> Option<(TextFrame, Arc<TextLayout>)> {
    geometry_in(app, edit, &StackCache::default())
}

fn layout_of(app: &App, edit: &TextEdit) -> Option<Arc<TextLayout>> {
    geometry(app, edit).map(|(_, layout)| layout)
}

/// How much of `edit`'s text shows at once, in canvas units: a Document's
/// window, and for a text that grows with its lines, the viewport.
fn page_height(app: &App, edit: &TextEdit) -> f32 {
    let rect = (app.document.entity(&edit.entity))
        .map(|entity| crate::scroll_follow::placed_rect(app, entity));
    match (edit.target, rect) {
        (Target::Note, Some(rect)) => frame::note_window(rect),
        (
            Target::Note
            | Target::Text
            | Target::Label
            | Target::Title
            | Target::EdgeLabel
            | Target::Comment
            | Target::Field,
            _,
        ) => app.session.viewport.y / app.session.camera.zoom.max(f32::EPSILON),
    }
}

/// Starts editing `id`'s text with all of it selected, if it has text to
/// edit, and makes it the selection. `created` says the entity was put in
/// the document for this session and has no undo step yet.
pub(crate) fn begin(app: &mut App, id: &EntityId, created: bool, effects: &mut Vec<Effect>) {
    end(app, effects);
    let Some(entity) = app.document.entity(id) else {
        return;
    };
    let Some((target, text)) = editable(app, entity) else {
        return;
    };
    let origin = Origin {
        text: text.to_owned(),
        rect: entity.rect,
        created,
    };
    let mut edit = TextEdit::new(id.clone(), target, text, origin);
    edit.active_ms = app.session.now_ms;
    if let Some(file) = note::file_of(entity).filter(|_| target == Target::Note) {
        // A Document is long: the caret starts at the top, with nothing
        // selected for the first key to replace.
        edit.select(0..0);
        edit.note = Some(note::NoteSave {
            file: file.to_owned(),
            changed_ms: None,
        });
    }
    app.session.editing = Some(edit);
    app.session.selection.set([ItemId::Entity(id.clone())]);
    effects.push(Effect::SetImeAllowed(true));
    if created {
        refit(app);
    }
    place_candidates(app, effects);
}

/// The entity the edit in progress placed for itself: it is in the document
/// with no undo step, so a change to it belongs to the step that ends the
/// session.
pub(crate) fn unrecorded(app: &App) -> Option<&EntityId> {
    let edit = app.session.editing.as_ref()?;
    (edit.origin.created && edit.target != Target::EdgeLabel).then_some(&edit.entity)
}

/// The entity whose text is being edited in place.
pub(crate) fn edited(app: &App) -> Option<&EntityId> {
    let edit = app.session.editing.as_ref()?;
    matches!(edit.target, Target::Text | Target::Label).then_some(&edit.entity)
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
    if edit.target == Target::Note {
        note::finish(app, edit, effects);
        return;
    }
    if edit.target == Target::EdgeLabel {
        return edge_label::end(app, &edit, effects);
    }
    if edit.target == Target::Comment {
        return comment::end(app, &edit, effects);
    }
    if edit.target == Target::Field {
        return field::end(app, &edit, effects);
    }
    let id = edit.entity.clone();
    let Some(entity) = app.document.entity(&id).cloned() else {
        return;
    };
    if edit.target == Target::Title {
        return title::end(app, &entity, &edit, effects);
    }
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
    // A Document's text is its file's and outlives the document: what has
    // not reached the file yet is written, not dropped.
    note::flush(app, effects);
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
    let text = title::single_line(edit.target, text);
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
pub(crate) fn place_candidates(app: &App, effects: &mut Vec<Effect>) {
    if let Some(caret) = app.caret_rect() {
        let camera = &app.session.camera;
        let corner = Vec2::new(caret.x as f32, caret.y as f32);
        effects.push(Effect::SetImeCursorArea {
            origin: camera.world_to_screen(corner),
            size: Vec2::new(caret.width as f32, caret.height as f32) * camera.zoom,
        });
    }
}
