//! The Documents file entities show: the text of each markdown file, and
//! how far each Document is scrolled.
//!
//! A Document is a file entity whose `.md` file lives in the space folder.
//! The file is the source of truth (ADR 0023): its text is not in the
//! `.canvas`, so `update` asks the shell for it with [`Effect::LoadNote`] and
//! the shell answers with [`Event::Note`](crate::Event::Note), once when it
//! has read the file and again each time the file changes on disk.

use std::collections::BTreeMap;
use std::sync::Arc;

use specular_doc::{Document, EntityId, Kind};

use crate::edit::{frame, note};
use crate::{App, Effect, Hit, WheelInput, hit_test};

/// What is known about one markdown file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteState {
    /// Asked for; no answer yet.
    Loading,
    /// The file's text as last read.
    Ready(Arc<str>),
    /// There is no file at the path.
    Missing,
    /// The file could not be read, or is not text.
    Failed,
}

/// What the shell reports about a markdown file it was asked to load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteNotice {
    /// The file's text now: the first read, or a change on disk.
    Text(String),
    /// There is no file at the path.
    Missing,
    /// The file could not be read, or is not text.
    Failed,
    /// An [`Effect::WriteNote`] was not carried out: the file holds `disk`,
    /// which is not what the shell last read from it or wrote to it.
    Refused {
        /// What the file holds.
        disk: String,
        /// What the write would have put there.
        ours: String,
    },
}

/// The markdown files asked for so far, by the `file` path as the document
/// writes it, and the scroll offset of each Document that has one.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Notes {
    by_file: BTreeMap<String, NoteState>,
    /// Canvas units each Document is scrolled down by, keyed by its entity.
    /// Two entities showing one file scroll apart.
    scroll: BTreeMap<EntityId, f32>,
    /// How tall the renderer found each Document's rows, in canvas units.
    heights: BTreeMap<EntityId, f32>,
}

impl Notes {
    pub(crate) fn get(&self, file: &str) -> Option<&NoteState> {
        self.by_file.get(file)
    }

    pub(crate) fn scroll(&self, entity: &EntityId) -> f32 {
        self.scroll.get(entity).copied().unwrap_or(0.0)
    }

    /// Makes `text` what is known of `file`, asked for or not.
    pub(crate) fn set_ready(&mut self, file: &str, text: &str) {
        (self.by_file).insert(file.to_owned(), NoteState::Ready(text.into()));
    }

    fn set_scroll(&mut self, entity: &EntityId, offset: f32) {
        if offset > 0.0 {
            self.scroll.insert(entity.clone(), offset);
        } else {
            self.scroll.remove(entity);
        }
    }
}

/// Whether `file` is shown as a Document: Electron's `MARKDOWN_EXTENSIONS`.
pub fn is_note_file(file: &str) -> bool {
    (file.rsplit_once('.')).is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("md"))
}

/// The file name of the `number`th Document called `base`: `base.md`, then
/// `base 2.md`, `base 3.md` and so on. The shell's note thread names a new
/// Document this way, and so does a note made through the API.
pub fn note_file_name(base: &str, number: u32) -> String {
    if number <= 1 {
        format!("{base}.md")
    } else {
        format!("{base} {number}.md")
    }
}

/// The markdown files `document` shows, each once per entity showing it.
fn wanted(document: &Document) -> impl Iterator<Item = &str> {
    document
        .entities()
        .filter_map(|entity| note_file(&entity.kind))
}

pub(crate) fn note_file(kind: &Kind) -> Option<&str> {
    match kind {
        Kind::File(file) if is_note_file(&file.file) => Some(file.file.as_str()),
        Kind::File(_)
        | Kind::Page(_)
        | Kind::Text(_)
        | Kind::Group(_)
        | Kind::Drawing(_)
        | Kind::Shape(_) => None,
    }
}

/// Asks for every markdown file the document shows that has not been asked
/// for. Files nothing shows any more are kept: an undo can bring them back.
pub(crate) fn request_new(app: &mut App, effects: &mut Vec<Effect>) {
    let notes = &mut app.session.notes;
    for file in wanted(&app.document) {
        if notes.by_file.contains_key(file) {
            continue;
        }
        notes.by_file.insert(file.to_owned(), NoteState::Loading);
        effects.push(Effect::LoadNote {
            file: file.to_owned(),
        });
    }
}

/// A different document was opened: lets go of the files it does not show
/// and the scroll of the entities it does not hold, and asks for the files
/// that are new.
pub(crate) fn reopen(app: &mut App, effects: &mut Vec<Effect>) {
    let document = &app.document;
    let notes = &mut app.session.notes;
    notes.by_file.retain(|file, _| {
        let shown = wanted(document).any(|wanted| wanted == file);
        if !shown {
            effects.push(Effect::DropNote { file: file.clone() });
        }
        shown
    });
    let shows_note = |entity: &EntityId| {
        (document.entity(entity)).is_some_and(|entity| note_file(&entity.kind).is_some())
    };
    notes.scroll.retain(|entity, _| shows_note(entity));
    notes.heights.retain(|entity, _| shows_note(entity));
    request_new(app, effects);
}

/// The shell answered for `file`. An answer for a file since let go of is
/// ignored, and so is a text that is already what is known of the file: the
/// shell's first read of a file just made, or one of our own writes.
pub(crate) fn on_notice(app: &mut App, file: &str, notice: NoteNotice, effects: &mut Vec<Effect>) {
    let Some(known) = app.session.notes.by_file.get(file) else {
        return;
    };
    let state = match notice {
        NoteNotice::Text(disk) => {
            if matches!(known, NoteState::Ready(text) if **text == *disk) {
                return;
            }
            if note::on_disk_change(app, file, &disk, effects) {
                return;
            }
            // An undo goes back from the text the file has now.
            if app.document.note(file).is_some() {
                note::hold(app, file, &disk);
            }
            NoteState::Ready(disk.into())
        }
        NoteNotice::Refused { disk, ours } => {
            note::keep_both(app, file, &disk, &ours, effects);
            return;
        }
        NoteNotice::Missing => NoteState::Missing,
        NoteNotice::Failed => NoteState::Failed,
    };
    app.session.notes.by_file.insert(file.to_owned(), state);
}

/// How tall the text of the Document `entity` is, when that is known: the
/// source's layout while it is edited, and otherwise what the renderer last
/// reported for its rows.
fn content_height(app: &App, entity: &EntityId) -> Option<f32> {
    match app.text_edit() {
        Some(edit) if edit.entity() == entity => app.editing_layout().map(|it| it.height()),
        _ => app.session.notes.heights.get(entity).copied(),
    }
}

/// The furthest the Document `entity` scrolls: until its last row reaches
/// the bottom of its window. `None` until the height of its text is known.
fn scroll_end(app: &App, entity: &EntityId) -> Option<f32> {
    let rect = app.document.entity(entity)?.rect;
    Some((content_height(app, entity)? - frame::note_window(rect)).max(0.0))
}

/// Scrolls the Document `entity` to `offset`, held between its top and its
/// end.
pub(crate) fn scroll_to(app: &mut App, entity: &EntityId, offset: f32) {
    let end = scroll_end(app, entity).unwrap_or(f32::MAX);
    app.session.notes.set_scroll(entity, offset.clamp(0.0, end));
}

/// The renderer measured the rows of the Documents it drew. A Document
/// scrolled past its end, because its text got shorter or its card taller,
/// comes back to it.
pub(crate) fn on_heights(app: &mut App, heights: Vec<(EntityId, f32)>) {
    for (entity, height) in heights {
        if app.document.entity(&entity).is_none() {
            continue;
        }
        app.session.notes.heights.insert(entity.clone(), height);
        let offset = app.session.notes.scroll(&entity);
        scroll_to(app, &entity, offset);
    }
}

/// Scrolls the Document being edited by the least that brings its caret
/// into view.
pub(crate) fn reveal_caret(app: &mut App) {
    let Some(edit) = app.text_edit().filter(|edit| edit.is_note()) else {
        return;
    };
    let entity = edit.entity().clone();
    let (Some(caret), Some(rect)) = (
        app.caret_rect(),
        app.document.entity(&entity).map(|it| it.rect),
    ) else {
        return;
    };
    let offset = app.session.notes.scroll(&entity);
    let window = frame::note_window(rect);
    // The caret's top and bottom inside the text, from its place on canvas.
    let top = (caret.y - rect.y) as f32 - frame::NOTE_PADDING + offset;
    let bottom = top + caret.height as f32;
    let wanted = if top < offset {
        top
    } else if bottom > offset + window {
        bottom - window
    } else {
        offset
    };
    scroll_to(app, &entity, wanted);
}

/// Scrolls the Document under the pointer when it is the whole selection,
/// and says whether it took the wheel. Cmd or Ctrl+wheel is a zoom and is
/// left for the canvas.
pub(crate) fn on_wheel(app: &mut App, input: &WheelInput) -> bool {
    if input.modifiers.meta || input.modifiers.control {
        return false;
    }
    let session = &app.session;
    let Some(Hit::EntityBody { entity }) = session.pointer.map(|at| hit_test(app, at)) else {
        return false;
    };
    let selected = session.selection.single_entity() == Some(&entity);
    let is_note = (app.document.entity(&entity)).is_some_and(|it| note_file(&it.kind).is_some());
    if !selected || !is_note {
        return false;
    }
    // Positive `y` moves content down, which is scrolling back up.
    let by = -input.delta.y / session.camera.zoom.max(f32::EPSILON);
    let offset = session.notes.scroll(&entity) + by;
    scroll_to(app, &entity, offset);
    true
}
