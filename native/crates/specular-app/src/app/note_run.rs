//! The Document effects: asking the note thread to read and watch a
//! markdown file, and telling the app what it read.

use std::path::Path;

use specular_interact::{Event, NoteNotice};

use super::Shell;
use super::image_run::space_folder;
use crate::notes::{NoteLoader, ReadFailure};

/// Starts the note thread for a document opened from `canvas`. Relative
/// paths start from the folder that file is in, its space folder.
pub(super) fn start_loader(canvas: Option<&Path>) -> Option<NoteLoader> {
    match NoteLoader::new(canvas.and_then(space_folder)) {
        Ok(loader) => Some(loader),
        Err(error) => {
            tracing::warn!("documents will not load: cannot start the note thread: {error}");
            None
        }
    }
}

impl Shell {
    pub(super) fn load_note(&self, file: &str) {
        if let Some(loader) = self.note_loader.as_ref() {
            loader.watch(file);
        }
    }

    pub(super) fn drop_note(&self, file: &str) {
        if let Some(loader) = self.note_loader.as_ref() {
            loader.unwatch(file);
        }
    }

    /// Tells the app about every file the note thread has read since the
    /// last turn. Text is cheap to hand over, so there is no cap per turn.
    pub(super) fn take_read_notes(&mut self) {
        while let Some(read) = self.note_loader.as_ref().and_then(NoteLoader::take) {
            let notice = match read.result {
                Ok(text) => NoteNotice::Text(text),
                Err(ReadFailure::Missing) => NoteNotice::Missing,
                Err(ReadFailure::Failed) => NoteNotice::Failed,
            };
            self.dispatch(Event::Note {
                file: read.file,
                notice,
            });
        }
    }
}
