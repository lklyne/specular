//! [`SpaceFiles`]: the `.canvas` files of every open project, one
//! [`Persistence`] a canvas, each in its project's canvas folder. Every canvas is followed, shown or not: a save
//! goes to whichever canvas changed, and a file another tool edits is read
//! again whether its canvas is the active one or in the background.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use specular_doc::Document;
use specular_interact::{App, CanvasId, Space, iso8601};

use super::listing;
use crate::persist::Persistence;

/// The files of the open projects.
#[derive(Debug)]
pub(crate) struct SpaceFiles {
    files: HashMap<CanvasId, Persistence>,
    /// The canvas folder each followed canvas's file is in, kept so a
    /// canvas already gone from the app can still be trashed.
    folders: HashMap<CanvasId, PathBuf>,
    /// Sends a file to the trash. A test puts its own in.
    trash: fn(&Path) -> Result<(), String>,
}

impl SpaceFiles {
    /// The files of the open projects, following none yet.
    pub(crate) fn new() -> Self {
        Self {
            files: HashMap::new(),
            folders: HashMap::new(),
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
            let Some(folder) = folder_of(space, &canvas.id) else {
                continue;
            };
            let path = folder.join(&canvas.file);
            self.folders.insert(canvas.id.clone(), folder);
            (self.files.entry(canvas.id.clone())).or_insert_with(|| Persistence::open(&path));
        }
    }

    /// Stops following every canvas the app no longer has: a project was
    /// closed. Its files were flushed first.
    pub(crate) fn prune(&mut self, space: &Space) {
        self.files.retain(|id, _| space.canvas(id).is_some());
        self.folders.retain(|id, _| space.canvas(id).is_some());
    }

    /// Where the canvas `id`'s file is.
    #[expect(
        clippy::unused_self,
        reason = "asked of the files, which only a run that writes has"
    )]
    pub(crate) fn path_of(&self, app: &App, id: &CanvasId) -> Option<PathBuf> {
        app.space().canvas_path(id).map(PathBuf::from)
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
        let (Some(path), Some(folder)) = (self.path_of(app, id), folder_of(app.space(), id)) else {
            return;
        };
        self.folders.insert(id.clone(), folder);
        (self.files.entry(id.clone()))
            .or_insert_with(|| Persistence::new_file(&path))
            .save(app, id);
    }

    /// The canvas `id` was renamed: moves its file from `from` to `to`.
    /// What is unsaved is written to the old name first, so the move
    /// carries it.
    pub(crate) fn rename(&mut self, app: &App, id: &CanvasId, from: &str, to: &str) {
        let Some(folder) = folder_of(app.space(), id) else {
            return;
        };
        let (old, new) = (folder.join(from), folder.join(to));
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
        let Some(folder) = self.folders.remove(id) else {
            return;
        };
        let path = folder.join(file);
        if !path.exists() {
            return;
        }
        if let Err(error) = (self.trash)(&path) {
            tracing::warn!(path = %path.display(), "canvas file not moved to the trash: {error}");
        }
    }

    /// Writes each project's index: which canvases it has, and which is
    /// active when the canvas showing is one of them.
    #[expect(
        clippy::unused_self,
        reason = "asked of the files, which only a run that writes has"
    )]
    pub(crate) fn save_meta(&self, app: &App) {
        let now = iso8601(app.session().now_ms);
        let space = app.space();
        for project in space.projects() {
            let folder = Path::new(&project.canvases);
            if let Err(error) = listing::write_meta(folder, space, &project.id, &now) {
                tracing::warn!(folder = %folder.display(), "project index not written: {error}");
            }
        }
    }
}

/// The canvas folder of the canvas `id`'s project.
fn folder_of(space: &Space, id: &CanvasId) -> Option<PathBuf> {
    let canvas = space.canvas(id)?;
    space
        .project(&canvas.project)
        .map(|project| PathBuf::from(&project.canvases))
}

fn system_trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|error| error.to_string())
}
