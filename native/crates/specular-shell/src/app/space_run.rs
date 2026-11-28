//! The space effects: opening the first project and the others beside it
//! (ADR 0062), and keeping their canvases' files in step with the app.

use std::path::{Path, PathBuf};

use specular_doc::Document;
use specular_interact::{Action, CanvasAction, CanvasId, Event, ProjectId};

use super::runtime::{Runtime, ShellWindow};
use super::{START_CAMERA, image_run, note_run};
use crate::prefs;
use crate::space::{self, SpaceFiles};

impl<W: ShellWindow> Runtime<W> {
    /// Shows the space at `folder` in place of what is open, which is
    /// saved first. `file` names the canvas file to show; without it the
    /// space's last active canvas is.
    pub(crate) fn open_space(&mut self, folder: &Path, file: Option<&str>) -> anyhow::Result<()> {
        let folder = std::path::absolute(folder)?;
        let saved = self.saved_projects();
        let opened = space::open(&folder, file, START_CAMERA, saved.git_of(&folder))?;
        self.flush_files();
        // Through an empty document, so nothing carries over: the same
        // relative path names another file in another space folder.
        self.dispatch(Event::DocumentOpened(Box::new(Document::new())));
        let canvas_folder = (opened.project.as_ref()).map_or_else(
            || folder.clone(),
            |project| PathBuf::from(&project.canvases),
        );
        self.space = None;
        self.enter_folder(canvas_folder);
        self.files = Some(SpaceFiles::new());
        self.dispatch(Event::SpaceOpened(Box::new(opened)));
        self.open_saved_projects(&saved);
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

    /// Shows the canvas file at `path`. A canvas of an open project is
    /// switched to, with its undo history and camera as they were left. A
    /// file in another folder opens that folder as a project beside the
    /// others and shows it; with no project open, it opens as the space.
    pub fn open_canvas_file(&mut self, path: &Path) -> anyhow::Result<()> {
        let (Some(folder), Some(file)) = (path.parent(), path.file_name()) else {
            return Ok(());
        };
        let file = file.to_string_lossy().into_owned();
        if let Some(canvas) = self.canvas_at(folder, &file) {
            self.dispatch(Event::Action(Action::Canvas(CanvasAction::Switch(canvas))));
            return Ok(());
        }
        if self.files.is_none() || self.app.space().projects().is_empty() {
            return self.open_space(folder, Some(&file));
        }
        self.open_project(folder);
        if let Some(canvas) = self.canvas_at(folder, &file) {
            self.dispatch(Event::Action(Action::Canvas(CanvasAction::Switch(canvas))));
        }
        Ok(())
    }

    /// The canvas kept in `file` in the canvas folder `folder`, if a
    /// project open has it.
    fn canvas_at(&self, folder: &Path, file: &str) -> Option<CanvasId> {
        let space = self.app.space();
        let project =
            (space.projects().iter()).find(|project| Path::new(&project.canvases) == folder)?;
        (space.canvases_of(&project.id))
            .find(|canvas| canvas.file == file)
            .map(|canvas| canvas.id.clone())
    }

    /// Opens `folder` as a project beside the ones open, unless it is one
    /// of them, and keeps the list for the next launch. A folder that
    /// cannot be read changes nothing.
    pub(crate) fn open_project(&mut self, folder: &Path) {
        if self.files.is_none() {
            return;
        }
        let Ok(folder) = std::path::absolute(folder) else {
            return;
        };
        let id = ProjectId::of_folder(&folder.to_string_lossy());
        if self.app.space().project(&id).is_some() {
            return;
        }
        let git = self.saved_projects().git_of(&folder);
        match space::open(&folder, None, START_CAMERA, git) {
            Ok(opened) => {
                self.dispatch(Event::ProjectOpened(Box::new(opened)));
                if let Some(files) = self.files.as_mut() {
                    files.follow(self.app.space());
                }
                tracing::info!(folder = %folder.display(), "opened project");
            }
            Err(error) => {
                tracing::warn!(folder = %folder.display(), "project not opened: {error:#}");
            }
        }
    }

    /// Opens the projects the last launch had beside the first, then,
    /// once, every connected repo that is not open yet: a repo is a
    /// project (ADR 0062). A folder that is gone is left out.
    fn open_saved_projects(&mut self, saved: &prefs::SavedProjects) {
        for folder in &saved.others {
            if folder.is_dir() {
                self.open_project(folder);
            }
        }
        if saved.from_repos {
            return;
        }
        let repos: Vec<PathBuf> = (self.app.repos().all().iter())
            .map(|repo| PathBuf::from(&repo.absolute_path))
            .collect();
        for folder in repos {
            if folder.is_dir() {
                self.open_project(&folder);
            }
        }
        if self.prefs.is_some() {
            self.save_projects(true);
        }
    }

    /// What the preferences keep of the projects. Empty for a run with no
    /// preferences file.
    fn saved_projects(&self) -> prefs::SavedProjects {
        (self.prefs.as_deref()).map_or_default(prefs::load_projects)
    }

    /// Keeps the projects open beside the first, and each one's git
    /// choice, for the next launch. `from_repos` marks the connected repos
    /// as opened already.
    pub(super) fn save_projects(&self, from_repos: bool) {
        let Some(path) = self.prefs.as_deref() else {
            return;
        };
        let mut saved = prefs::load_projects(path);
        let projects = self.app.space().projects();
        saved.others = (projects.iter().skip(1))
            .map(|project| PathBuf::from(&project.folder))
            .collect();
        for project in projects {
            saved.git.retain(|(folder, _)| *folder != project.folder);
            if let Some(choice) = project.git {
                saved.git.push((project.folder.clone(), choice));
            }
        }
        saved.from_repos |= from_repos;
        if let Err(error) = prefs::save_projects(path, &saved) {
            tracing::warn!(path = %path.display(), "projects not saved: {error}");
        }
    }

    /// Writes a repo project's `.specular/.gitignore` for its git choice.
    pub(super) fn write_git_choice(&self, id: &ProjectId) {
        let Some(project) = self.app.space().project(id) else {
            return;
        };
        if let Err(error) = space::write_git_choice(project) {
            tracing::warn!(folder = %project.folder, "git choice not written: {error}");
        }
    }

    /// The canvas showing is in another project now: relative paths start
    /// from its canvas folder.
    pub(super) fn project_entered(&mut self) {
        if let Some(folder) = self.app.space().active_project() {
            let folder = PathBuf::from(&folder.canvases);
            self.enter_folder(folder);
        }
    }

    /// Makes `folder` the one relative file paths start from, and where
    /// pasted and dropped files go, with image and note loaders of its
    /// own.
    fn enter_folder(&mut self, folder: PathBuf) {
        if self.space.as_ref() == Some(&folder) {
            return;
        }
        self.image_loader = image_run::start_loader(Some(folder.clone()));
        self.note_loader = note_run::start_loader(Some(folder.clone()));
        self.space = Some(folder);
    }

    /// Remembers `folder` as the space to open when the Electron app's
    /// settings name none.
    pub(crate) fn remember_space(&self, folder: &Path) {
        let Some(path) = self.prefs.as_deref() else {
            return;
        };
        if let Err(error) = prefs::save_space_path(path, folder) {
            tracing::warn!(path = %path.display(), "space folder not remembered: {error}");
        }
    }

    /// Writes every unsaved canvas without waiting for the autosave.
    pub fn flush_files(&mut self) {
        if let Some(files) = self.files.as_mut() {
            files.flush(&self.app);
        }
    }

    /// Writes what is unsaved in the project `id`, takes it out of the
    /// list, and stops following its canvases' files.
    pub(super) fn close_project(&mut self, id: &ProjectId) {
        if let Some(files) = self.files.as_mut() {
            files.flush(&self.app);
        }
        self.dispatch(Event::ProjectClosed(id.clone()));
        if let Some(files) = self.files.as_mut() {
            files.prune(self.app.space());
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
