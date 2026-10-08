//! The space effects: opening a space folder, and keeping its canvases'
//! files in step with the app.

use std::path::Path;

use specular_doc::Document;
use specular_interact::{CanvasId, Event};

use super::{START_CAMERA, Shell, image_run, note_run};
use crate::prefs;
use crate::space::{self, SpaceFiles};

impl Shell {
    /// Shows the space at `folder` in place of what is open, which is
    /// saved first. `file` names the canvas file to show; without it the
    /// space's last active canvas is.
    pub(super) fn open_space(&mut self, folder: &Path, file: Option<&str>) -> anyhow::Result<()> {
        let folder = std::path::absolute(folder)?;
        let opened = space::open(&folder, file, START_CAMERA)?;
        self.flush_files();
        // Through an empty document, so nothing carries over: the same
        // relative path names another file in another space folder.
        self.dispatch(Event::DocumentOpened(Box::new(Document::new())));
        self.image_loader = image_run::start_loader(Some(folder.clone()));
        self.note_loader = note_run::start_loader(Some(folder.clone()));
        self.files = Some(SpaceFiles::new(&folder));
        self.space = Some(folder.clone());
        self.dispatch(Event::SpaceOpened(Box::new(opened)));
        if let Some(files) = self.files.as_mut() {
            files.follow(self.app.space());
        }
        tracing::info!(
            folder = %folder.display(),
            canvases = self.app.space().canvases().len(),
            active = %self.app.space().active().name,
            "opened space"
        );
        Ok(())
    }

    /// Remembers `folder` as the space to open when the Electron app's
    /// settings name none.
    pub(super) fn remember_space(&self, folder: &Path) {
        let Some(path) = self.prefs.as_deref() else {
            return;
        };
        if let Err(error) = prefs::save_space_path(path, folder) {
            tracing::warn!(path = %path.display(), "space folder not remembered: {error}");
        }
    }

    /// Writes every unsaved canvas without waiting for the autosave.
    pub(super) fn flush_files(&mut self) {
        if let Some(files) = self.files.as_mut() {
            files.flush(&self.app);
        }
    }

    /// The active canvas changed: save it once changes stop.
    pub(super) fn request_save(&mut self) {
        let active = self.app.space().active().id.clone();
        if let Some(files) = self.files.as_mut() {
            files.request_save(&active);
        }
    }

    pub(super) fn write_canvas(&mut self, canvas: &CanvasId) {
        if let Some(files) = self.files.as_mut() {
            files.write_now(&self.app, canvas);
        }
    }

    pub(super) fn rename_canvas_file(&mut self, canvas: &CanvasId, from: &str, to: &str) {
        if let Some(files) = self.files.as_mut() {
            files.rename(&self.app, canvas, from, to);
        }
    }

    pub(super) fn trash_canvas_file(&mut self, canvas: &CanvasId, file: &str) {
        if let Some(files) = self.files.as_mut() {
            files.trash(canvas, file);
        }
    }

    pub(super) fn save_space_meta(&self) {
        if let Some(files) = self.files.as_ref() {
            files.save_meta(&self.app);
        }
    }

    /// Writes the autosaves that are due, and puts each canvas whose file
    /// another tool edited back in step with it. The camera stays, and
    /// `update` drops whatever the selection named that the new document
    /// lacks.
    pub(super) fn sync_files(&mut self) {
        let changed = match self.files.as_mut() {
            Some(files) => files.turn(&self.app),
            None => return,
        };
        for (canvas, document) in changed {
            self.dispatch(Event::CanvasFileChanged {
                canvas,
                document: Box::new(document),
            });
        }
    }

    /// Whether the active canvas has a change that is not in its file yet.
    pub(super) fn active_unsaved(&self) -> bool {
        let active = &self.app.space().active().id;
        (self.files.as_ref()).is_some_and(|files| files.has_unsaved(active))
    }
}
