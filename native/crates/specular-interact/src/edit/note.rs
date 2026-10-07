//! Editing a Document: a markdown file's source, in place.
//!
//! The file is the source of truth (ADR 0023). While the edit is open the
//! working text is written to it through [`Effect::WriteNote`], a third of a
//! second after the last change. When the edit ends, the document takes the
//! new text as one undo step, so Command+Z after an edit puts the old text
//! back in the file instead of undoing whatever came before it.
//!
//! Two writers can meet: the file changes on disk while it is being edited,
//! or the shell refuses a write because the file is no longer what it last
//! read. Neither text is dropped. The edit keeps the file, and the other
//! text is saved beside it as a conflict copy with a Document of its own.

use specular_doc::{Command, Entity, EntityId, FileRef, ItemId, Kind, Rect};

use super::buffer::{Target, TextEdit};
use crate::notes::{self, NoteState};
use crate::{App, Effect, update};

/// How long after the last change the working text is written to its file.
const SAVE_DELAY_MS: u64 = 350;
/// Canvas units between a Document and the conflict copy put beside it.
const COPY_GAP: f64 = 20.0;

/// The file an edit of a Document writes to, and whether it is behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoteSave {
    /// The file, as the entity names it.
    pub(crate) file: String,
    /// When the text last changed, if it has since it was last written.
    pub(crate) changed_ms: Option<u64>,
}

/// The working text changed: it is owed a write.
pub(crate) fn touch(app: &mut App) {
    let now = app.session.now_ms;
    if let Some(save) = (app.session.editing.as_mut()).and_then(|edit| edit.note.as_mut()) {
        save.changed_ms = Some(now);
    }
}

/// Writes `text` to `file`, and shows it at once wherever the file is shown.
fn write(app: &mut App, file: &str, text: &str, effects: &mut Vec<Effect>) {
    app.session.notes.set_ready(file, text);
    effects.push(Effect::WriteNote {
        file: file.to_owned(),
        text: text.to_owned(),
    });
}

/// The clock moved: writes the working text once it has been still for the
/// save delay. A composition is left to finish first.
pub(crate) fn autosave(app: &mut App, effects: &mut Vec<Effect>) {
    let now = app.session.now_ms;
    let due = (app.session.editing.as_ref()).is_some_and(|edit| {
        let changed = edit.note.as_ref().and_then(|save| save.changed_ms);
        edit.composition.is_none()
            && changed.is_some_and(|at| now.saturating_sub(at) >= SAVE_DELAY_MS)
    });
    if due {
        flush(app, effects);
    }
}

/// Writes the working text of the Document being edited, if its file is
/// behind.
pub(crate) fn flush(app: &mut App, effects: &mut Vec<Effect>) {
    let Some(edit) = &mut app.session.editing else {
        return;
    };
    let Some(save) = (edit.note.as_mut()).filter(|save| save.changed_ms.is_some()) else {
        return;
    };
    save.changed_ms = None;
    let (file, text) = (save.file.clone(), edit.text.clone());
    write(app, &file, &text, effects);
}

/// The edit of a Document ended: the file gets the text if it is behind, and
/// a text that differs from the one the edit started with is one undo step.
pub(crate) fn finish(app: &mut App, edit: TextEdit, effects: &mut Vec<Effect>) {
    let Some(save) = edit.note else {
        return;
    };
    if save.changed_ms.is_some() {
        write(app, &save.file, &edit.text, effects);
    }
    if edit.text == edit.origin.text {
        return;
    }
    // The step's inverse has to put back the text the edit started with, so
    // the document holds that first, with no step of its own.
    hold(app, &save.file, &edit.origin.text);
    let step = Command::SetNote {
        file: save.file,
        text: Some(edit.text),
    };
    update::document_step(app, step, effects);
}

/// Makes `text` what the document holds for `file`, with no undo step.
pub(crate) fn hold(app: &mut App, file: &str, text: &str) {
    if app.document.note(file) == Some(text) {
        return;
    }
    let command = Command::SetNote {
        file: file.to_owned(),
        text: Some(text.to_owned()),
    };
    if let Err(error) = app.document.apply(command) {
        tracing::warn!("note text refused: {error}");
    }
}

/// The texts the document holds, to compare across an undo or a redo with
/// [`write_stepped`].
pub(crate) fn held(app: &App) -> Vec<(String, String)> {
    (app.document.notes())
        .map(|(file, text)| (file.to_owned(), text.to_owned()))
        .collect()
}

/// An undo or a redo ran: every file whose held text it changed is written.
pub(crate) fn write_stepped(app: &mut App, before: &[(String, String)], effects: &mut Vec<Effect>) {
    for (file, text) in held(app) {
        let same = before.iter().any(|(was, old)| *was == file && *old == text);
        if !same {
            write(app, &file, &text, effects);
        }
    }
}

/// The file changed on disk to `theirs`. Returns whether that is a conflict
/// with the open edit, which keeps the file: `theirs` goes to a copy.
pub(crate) fn on_disk_change(
    app: &mut App,
    file: &str,
    theirs: &str,
    effects: &mut Vec<Effect>,
) -> bool {
    let Some(edit) = (app.session.editing.as_mut()).filter(|edit| is_of(edit, file)) else {
        return false;
    };
    if edit.text == theirs {
        // The same text from both sides: nothing is owed.
        if let Some(save) = &mut edit.note {
            save.changed_ms = None;
        }
        return false;
    }
    let ours = edit.text.clone();
    keep_both(app, file, theirs, &ours, effects);
    true
}

fn is_of(edit: &TextEdit, file: &str) -> bool {
    edit.target == Target::Note && edit.note.as_ref().is_some_and(|save| save.file == file)
}

/// Two texts for one file. `ours` takes the file, and `theirs` is written
/// beside it as a conflict copy, with a Document of its own next to the
/// first entity that shows the file.
pub(crate) fn keep_both(
    app: &mut App,
    file: &str,
    theirs: &str,
    ours: &str,
    effects: &mut Vec<Effect>,
) {
    let tag = app.fresh_id();
    let copy = conflict_name(file, tag.get(..6).unwrap_or(&tag));
    effects.push(Effect::WriteNote {
        file: copy.clone(),
        text: theirs.to_owned(),
    });
    let beside = (app.document.entities()).find_map(|entity| match &entity.kind {
        Kind::File(shown) if shown.file == file => Some((entity.rect, shown.clone())),
        Kind::File(_)
        | Kind::Page(_)
        | Kind::Text(_)
        | Kind::Group(_)
        | Kind::Drawing(_)
        | Kind::Shape(_) => None,
    });
    if let Some((rect, shown)) = beside {
        let id = EntityId::from(app.fresh_id().as_str());
        let rect = rect.translated(rect.width + COPY_GAP, 0.0);
        let kind = Kind::File(FileRef {
            file: copy,
            ..shown
        });
        let command = Command::InsertEntity {
            entity: Box::new(Entity::new(id, rect, kind)),
            at: app.document.stack_len(),
        };
        update::document_step(app, command, effects);
    } else {
        tracing::warn!("{file} changed under an edit; the other text is in {copy}");
    }
    if let Some(save) = (app.session.editing.as_mut())
        .filter(|edit| is_of(edit, file))
        .and_then(|edit| edit.note.as_mut())
    {
        save.changed_ms = None;
    }
    write(app, file, ours, effects);
}

/// `plan.md` as `plan (conflict a1b2c3).md`, in the same folder.
fn conflict_name(file: &str, tag: &str) -> String {
    let (stem, extension) = match file.rsplit_once('.') {
        Some((stem, extension)) if !extension.contains('/') => (stem, extension),
        _ => (file, "md"),
    };
    format!("{stem} (conflict {tag}).{extension}")
}

/// The shell made an empty markdown file for the add-document tool: a
/// Document for it goes at `rect` as one undo step, and is edited at once.
pub(crate) fn created(app: &mut App, file: String, rect: Rect, effects: &mut Vec<Effect>) {
    super::end(app, effects);
    // The file is known to be empty, so the edit need not wait for a read.
    // The load still goes out, to have the file watched.
    app.session.notes.set_ready(&file, "");
    effects.push(Effect::LoadNote { file: file.clone() });
    let id = EntityId::from(app.fresh_id().as_str());
    let kind = Kind::File(FileRef {
        file,
        subpath: None,
        object_fit: None,
        preset_index: None,
        metadata: None,
    });
    crate::live::create(app, Entity::new(id.clone(), rect, kind), effects);
    app.session.selection.set([ItemId::Entity(id.clone())]);
    super::begin(app, &id, false, effects);
}

/// Whether `state` is a text that can be edited.
pub(crate) fn text_of(state: Option<&NoteState>) -> Option<&str> {
    match state {
        Some(NoteState::Ready(text)) => Some(text),
        Some(NoteState::Loading | NoteState::Missing | NoteState::Failed) | None => None,
    }
}

/// The markdown file `entity` shows as a Document, if it is one.
pub(crate) fn file_of(entity: &Entity) -> Option<&str> {
    notes::note_file(&entity.kind)
}

#[cfg(test)]
mod tests {
    use super::conflict_name;

    #[test]
    fn a_conflict_copy_sits_beside_its_file_with_the_same_extension() {
        assert_eq!(
            conflict_name("plan.md", "a1b2c3"),
            "plan (conflict a1b2c3).md"
        );
        assert_eq!(
            conflict_name("notes/v1.2/Plan.MD", "ff"),
            "notes/v1.2/Plan (conflict ff).MD"
        );
        assert_eq!(
            conflict_name("notes.d/plan", "ff"),
            "notes.d/plan (conflict ff).md"
        );
    }
}
