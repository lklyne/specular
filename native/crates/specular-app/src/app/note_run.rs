//! The Document effects: asking the note thread to read and watch a
//! markdown file, and telling the app what it read.

use std::path::PathBuf;

use specular_doc::Rect;
use specular_interact::{Action, Event, NoteNotice};

use super::runtime::{Runtime, ShellWindow};
use crate::notes::{NoteLoader, NoteOutcome, ReadFailure};

/// Starts the note thread for the space at `space`, the folder relative
/// paths start from. With none, only absolute paths resolve.
pub(super) fn start_loader(space: Option<PathBuf>) -> Option<NoteLoader> {
    match NoteLoader::new(space) {
        Ok(loader) => Some(loader),
        Err(error) => {
            tracing::warn!("documents will not load: cannot start the note thread: {error}");
            None
        }
    }
}

impl<W: ShellWindow> Runtime<W> {
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

    pub(super) fn write_note(&self, file: String, text: String) {
        if let Some(loader) = self.note_loader.as_ref() {
            loader.write(file, text);
        } else {
            tracing::warn!(file, "no note thread; the document is not written");
        }
    }

    pub(super) fn create_note(&self, rect: Rect) {
        if let Some(loader) = self.note_loader.as_ref() {
            loader.create(rect);
        } else {
            tracing::warn!("no note thread; no document made");
        }
    }

    /// Ends an open edit so its text is written, waits for every write, and
    /// stops the note thread. For the way out.
    pub(super) fn finish_notes(&mut self) {
        if self.app.text_edit().is_some() {
            self.dispatch(Event::Action(Action::Cancel));
        }
        if let Some(loader) = self.note_loader.take() {
            loader.finish();
        }
    }

    /// Tells the app how tall the rows of each Document on screen came out,
    /// when that has changed since it was last told.
    pub(super) fn report_note_heights(&mut self) {
        let Some(gpu) = self.gpu.as_ref() else {
            return;
        };
        let changed: Vec<_> = (gpu.compositor().column_heights().iter())
            .filter(|(entity, height)| self.note_heights.get(entity) != Some(height))
            .cloned()
            .collect();
        if changed.is_empty() {
            return;
        }
        self.note_heights.extend(changed.iter().cloned());
        self.dispatch(Event::NoteHeights(changed));
    }

    /// Tells the app about every file the note thread has read since the
    /// last turn, every write it refused and every file it made. Text is cheap to hand over, so there is no cap per turn.
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
        while let Some(outcome) = self.note_loader.as_ref().and_then(NoteLoader::take_outcome) {
            self.dispatch(match outcome {
                NoteOutcome::Refused { file, disk, ours } => Event::Note {
                    file,
                    notice: NoteNotice::Refused { disk, ours },
                },
                NoteOutcome::Created { file, rect } => Event::NoteCreated { file, rect },
            });
        }
    }
}
