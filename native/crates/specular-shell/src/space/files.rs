//! [`SpaceFiles`]: the `.canvas` files of the open space, one
//! [`Persistence`] a canvas. Every canvas is followed, shown or not: a save
//! goes to whichever canvas changed, and a file another tool edits is read
//! again whether its canvas is the active one or in the background.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use specular_doc::Document;
use specular_interact::{App, CanvasId, Space, iso8601};

use super::listing;
use crate::persist::Persistence;

/// The files of the open space.
#[derive(Debug)]
pub(crate) struct SpaceFiles {
    folder: PathBuf,
    files: HashMap<CanvasId, Persistence>,
    /// Sends a file to the trash. A test puts its own in.
    trash: fn(&Path) -> Result<(), String>,
}

impl SpaceFiles {
    /// The files of the space at `folder`, following none yet.
    pub(crate) fn new(folder: &Path) -> Self {
        Self {
            folder: folder.to_owned(),
            files: HashMap::new(),
            trash: system_trash,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_trash(mut self, trash: fn(&Path) -> Result<(), String>) -> Self {
        self.trash = trash;
        self
    }

    /// Starts following the file of every canvas of `space` that is not
    /// followed yet. Each was just read.
    pub(crate) fn follow(&mut self, space: &Space) {
        for canvas in space.canvases() {
            let path = self.folder.join(&canvas.file);
            (self.files.entry(canvas.id.clone())).or_insert_with(|| Persistence::open(&path));
        }
    }

    /// The canvas `id` changed: save it once changes stop.
    pub(crate) fn request_save(&mut self, id: &CanvasId) {
        if let Some(file) = self.files.get_mut(id) {
            file.request_save();
        }
    }

    /// Whether the canvas `id` has a change waiting to be written.
    pub(crate) fn has_unsaved(&self, id: &CanvasId) -> bool {
        self.files.get(id).is_some_and(Persistence::has_unsaved)
    }

    /// One loop turn: writes the saves that are due and looks at each file
    /// for an edit from outside. Returns the canvases whose file changed,
    /// each with the document to put in its place. Only the active canvas
    /// can have a drag in flight, which holds its save and its reload back.
    pub(crate) fn turn(&mut self, app: &App) -> Vec<(CanvasId, Document)> {
        let dragging = app.session().gesture.is_some();
        let active = &app.space().active().id;
        let mut changed = Vec::new();
        for (id, file) in &mut self.files {
            if let Some(document) = file.turn(app, id, dragging && id == active) {
                changed.push((id.clone(), document));
            }
        }
        changed
    }

    /// Writes every pending save now: the app is closing, another space is
    /// being opened, or Save was chosen.
    pub(crate) fn flush(&mut self, app: &App) {
        for (id, file) in &mut self.files {
            file.flush(app, id);
        }
    }

    /// Writes the canvas `id` now, making its file if it has none.
    pub(crate) fn write_now(&mut self, app: &App, id: &CanvasId) {
        let Some(canvas) = app.space().canvas(id) else {
            return;
        };
        let path = self.folder.join(&canvas.file);
        (self.files.entry(id.clone()))
            .or_insert_with(|| Persistence::new_file(&path))
            .save(app, id);
    }

    /// The canvas `id` was renamed: moves its file from `from` to `to`.
    /// What is unsaved is written to the old name first, so the move
    /// carries it.
    pub(crate) fn rename(&mut self, app: &App, id: &CanvasId, from: &str, to: &str) {
        let (old, new) = (self.folder.join(from), self.folder.join(to));
        let Some(file) = self.files.get_mut(id) else {
            return;
        };
        file.flush(app, id);
        if new.exists() {
            tracing::warn!(to = %new.display(), "not renamed: a file with that name is there");
            return;
        }
        match std::fs::rename(&old, &new) {
            Ok(()) => file.moved_to(&new),
            Err(error) => {
                tracing::warn!(from = %old.display(), "canvas file not renamed: {error}");
                // The canvas now goes by the new name, so its next save
                // makes the file there.
                *file = Persistence::new_file(&new);
                file.save(app, id);
            }
        }
    }

    /// The canvas `id` was deleted: stops following its file and sends it
    /// to the trash.
    pub(crate) fn trash(&mut self, id: &CanvasId, file: &str) {
        self.files.remove(id);
        let path = self.folder.join(file);
        if !path.exists() {
            return;
        }
        if let Err(error) = (self.trash)(&path) {
            tracing::warn!(path = %path.display(), "canvas file not moved to the trash: {error}");
        }
    }

    /// Writes the space's index: which canvases there are and which is
    /// active.
    pub(crate) fn save_meta(&self, app: &App) {
        let now = iso8601(app.session().now_ms);
        if let Err(error) = listing::write_meta(&self.folder, app.space(), &now) {
            tracing::warn!(folder = %self.folder.display(), "space index not written: {error}");
        }
    }
}

fn system_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|error| error.to_string())
}
