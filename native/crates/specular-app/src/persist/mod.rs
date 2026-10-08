//! Keeping one `.canvas` file and its canvas in step: autosave, and
//! reloading when another tool edits the file.
//!
//! [`FileSync`] decides; this module does the file calls it asks for. A
//! space has one [`Persistence`] a canvas (see `space::files`).

mod app_state;
mod disk;
mod file_sync;

use std::path::{Path, PathBuf};
use std::time::Instant;

use specular_doc::Document;
use specular_interact::{App, CanvasId};

pub(crate) use self::app_state::{camera_of, canvas_text};
pub(crate) use self::disk::{stamp, write_atomic};
pub(crate) use self::file_sync::Stamp;
use self::file_sync::{DiskChange, FileSync, Step};

/// The `.canvas` file of one canvas of the space.
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

    /// Follows a file at `path` that is not there yet: a new canvas. The
    /// first save makes it.
    pub(crate) fn new_file(path: &Path) -> Self {
        Self {
            path: path.to_owned(),
            sync: FileSync::new(String::new(), None),
            started: Instant::now(),
        }
    }

    /// The file was moved to `path`.
    pub(crate) fn moved_to(&mut self, path: &Path) {
        path.clone_into(&mut self.path);
        self.sync.restamp(disk::stamp(path));
    }

    /// The canvas changed: save it once changes stop.
    pub(crate) fn request_save(&mut self) {
        self.sync.request_save(self.now_ms());
    }

    /// One loop turn for the canvas `id`: saves when a save is due,
    /// otherwise looks for an edit from outside now and then. Returns a
    /// document to put in place of the canvas's when the file changed and
    /// nothing of ours is unsaved. While `dragging`, nothing happens.
    pub(crate) fn turn(&mut self, app: &App, id: &CanvasId, dragging: bool) -> Option<Document> {
        match self.sync.step(self.now_ms(), dragging) {
            Step::Idle => None,
            Step::Save => {
                self.save(app, id);
                None
            }
            Step::CheckDisk => self.check_disk(),
        }
    }

    /// Whether a change is waiting to be written.
    pub(crate) fn has_unsaved(&self) -> bool {
        self.sync.has_unsaved()
    }

    /// Writes a pending save now: the app is closing, or Save was chosen.
    pub(crate) fn flush(&mut self, app: &App, id: &CanvasId) {
        if self.sync.take_unsaved() {
            self.save(app, id);
        }
    }

    /// Writes the canvas `id` to the file now, pending save or not.
    pub(crate) fn save(&mut self, app: &App, id: &CanvasId) {
        self.sync.take_unsaved();
        let path = &self.path;
        let Some((document, camera)) = app.canvas_to_save(id) else {
            return;
        };
        let written = app_state::canvas_text(&document, camera)
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
