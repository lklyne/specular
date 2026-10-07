//! The File menu's commands: open another canvas, save now, close.

use std::path::Path;

use specular_doc::Document;
use specular_interact::{Action, Event};

use super::menu_bar::ShellCommand;
use super::{START_CAMERA, Shell, image_run, note_run};
use crate::persist::{self, Persistence};
use crate::scene;

impl Shell {
    pub(super) fn run_shell_command(&mut self, command: ShellCommand) {
        match command {
            ShellCommand::Open => self.choose_canvas(),
            ShellCommand::Save => self.save_now(),
            ShellCommand::Close | ShellCommand::Quit => self.exit(),
        }
    }

    /// Writes unsaved changes without waiting for the autosave.
    fn save_now(&mut self) {
        if let Some(persist) = self.persist.as_mut() {
            persist.flush(&self.app);
        }
    }

    /// Asks for a `.canvas` file and opens it. A file that cannot be opened
    /// leaves the current canvas as it is.
    fn choose_canvas(&mut self) {
        let mut dialog = rfd::FileDialog::new().add_filter("Canvas", &["canvas"]);
        if let Some(space) = self.space.as_deref() {
            dialog = dialog.set_directory(space);
        }
        if let Some(gpu) = self.gpu.as_ref() {
            dialog = dialog.set_parent(&*gpu.window);
        }
        let Some(path) = dialog.pick_file() else {
            return;
        };
        if let Err(error) = self.open_canvas(&path) {
            tracing::error!("{error:#}");
        }
    }

    /// Shows the canvas at `path` in place of the current one, which is
    /// saved first.
    fn open_canvas(&mut self, path: &Path) -> anyhow::Result<()> {
        let document = scene::load_document(Some(path), 0, 0)?;
        self.save_now();
        // Through an empty document, so nothing carries over: the same
        // relative path names another file in another space folder.
        self.dispatch(Event::DocumentOpened(Box::new(Document::new())));
        self.space = image_run::space_folder(path);
        self.image_loader = image_run::start_loader(Some(path));
        self.note_loader = note_run::start_loader(Some(path));
        self.persist = Some(Persistence::open(path));
        self.options.canvas = Some(path.to_owned());
        let camera = persist::camera_of(&document).unwrap_or(START_CAMERA);
        self.dispatch(Event::Action(Action::SetCamera(camera)));
        self.dispatch(Event::DocumentOpened(Box::new(document)));
        tracing::info!(path = %path.display(), "opened");
        Ok(())
    }
}
