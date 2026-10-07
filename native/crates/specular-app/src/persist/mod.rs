//! Keeping the open `.canvas` file and the document in step: autosave, and
//! reloading when another tool edits the file.
//!
//! [`FileSync`] decides; this module does the file calls it asks for.

mod app_state;
mod disk;
mod file_sync;

use std::path::{Path, PathBuf};
use std::time::Instant;

use specular_doc::Document;
use specular_interact::App;

pub(crate) use self::app_state::camera_of;
pub(crate) use self::disk::stamp;
pub(crate) use self::file_sync::Stamp;
use self::file_sync::{DiskChange, FileSync, Step};

/// The `.canvas` file the document was opened from.
#[derive(Debug)]
pub(crate) struct Persistence {
    path: PathBuf,
    sync: FileSync,
    /// The zero of the clock [`FileSync`] is given.
    started: Instant,
}

impl Persistence {
    /// Follows the file at `path`, which was just loaded.
    pub(crate) fn open(path: &Path) -> Self {
        // Read again rather than threaded through from the loader: if the
        // file changed in between, the first check sees a moved stamp.
        let stamp = disk::stamp(path);
        let text = std::fs::read_to_string(path).unwrap_or_default();
        Self {
            path: path.to_owned(),
            sync: FileSync::new(text, stamp),
            started: Instant::now(),
        }
    }

    /// The document changed: save it once changes stop.
    pub(crate) fn request_save(&mut self) {
        self.sync.request_save(self.now_ms());
    }

    /// One loop turn: saves when a save is due, otherwise looks for an edit
    /// from outside now and then. Returns a document to open in place of
    /// `app`'s when the file changed and nothing of ours is unsaved.
    pub(crate) fn turn(&mut self, app: &App) -> Option<Document> {
        let dragging = app.session().gesture.is_some();
        match self.sync.step(self.now_ms(), dragging) {
            Step::Idle => None,
            Step::Save => {
                self.save(app);
                None
            }
            Step::CheckDisk => self.check_disk(),
        }
    }

    /// Writes a pending save now, because the app is closing.
    pub(crate) fn flush(&mut self, app: &App) {
        if self.sync.take_unsaved() {
            self.save(app);
        }
    }

    fn save(&mut self, app: &App) {
        let path = &self.path;
        let written = app_state::canvas_text(app.document(), app.session().camera)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                disk::write_atomic(path, &text).map_err(|error| error.to_string())?;
                Ok(text)
            });
        match written {
            Ok(text) => {
                tracing::debug!(path = %path.display(), bytes = text.len(), "saved");
                self.sync.saved(text, disk::stamp(path));
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), "save failed, will retry: {error}");
                self.sync.save_failed(self.now_ms());
            }
        }
    }

    fn check_disk(&mut self) -> Option<Document> {
        let path = &self.path;
        let stamp = disk::stamp(path)?;
        if !self.sync.disk_stamp(stamp) {
            return None;
        }
        // An unreadable file is left for the next check: another writer may
        // be halfway through replacing it.
        let text = std::fs::read_to_string(path).ok()?;
        match self.sync.disk_text(&text, stamp) {
            DiskChange::Unchanged => None,
            DiskChange::KeepOurs => {
                tracing::warn!(
                    path = %path.display(),
                    "file changed on disk while there are unsaved changes; keeping ours"
                );
                None
            }
            DiskChange::Reload => match Document::from_canvas_str(&text) {
                Ok(document) => {
                    tracing::info!(path = %path.display(), "file changed on disk; reloading");
                    Some(document)
                }
                Err(error) => {
                    tracing::warn!(
                        path = %path.display(),
                        "file changed on disk but cannot be read; keeping ours: {error}"
                    );
                    None
                }
            },
        }
    }

    fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }
}
