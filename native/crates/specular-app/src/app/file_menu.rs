//! The File menu's commands: open a space or a canvas, save now, rename
//! the canvas, close.

use std::path::Path;

use specular_interact::{Action, CanvasAction, Event};

use super::Shell;
use super::menu_bar::ShellCommand;

impl Shell {
    pub(super) fn run_shell_command(&mut self, command: ShellCommand) {
        match command {
            ShellCommand::OpenSpace => self.choose_space(),
            ShellCommand::Open => self.choose_canvas(),
            ShellCommand::Save => self.flush_files(),
            ShellCommand::RenameCanvas => self.choose_canvas_name(),
            ShellCommand::Close | ShellCommand::Quit => self.exit(),
        }
    }

    fn dialog(&self) -> rfd::FileDialog {
        let mut dialog = rfd::FileDialog::new();
        if let Some(space) = self.space.as_deref() {
            dialog = dialog.set_directory(space);
        }
        if let Some(gpu) = self.gpu.as_ref() {
            dialog = dialog.set_parent(&*gpu.window);
        }
        dialog
    }

    /// Asks for a folder and opens it as the space. The choice is
    /// remembered for the next launch.
    fn choose_space(&mut self) {
        let Some(folder) = self.dialog().set_title("Open space").pick_folder() else {
            return;
        };
        match self.open_space(&folder, None) {
            Ok(()) => self.remember_space(&folder),
            Err(error) => tracing::error!("{error:#}"),
        }
    }

    /// Asks for a `.canvas` file and shows it, in the space its folder is.
    /// A file that cannot be opened leaves the current space as it is.
    fn choose_canvas(&mut self) {
        let dialog = self.dialog().add_filter("Canvas", &["canvas"]);
        let Some(path) = dialog.pick_file() else {
            return;
        };
        let (Some(folder), Some(file)) = (path.parent(), path.file_name()) else {
            return;
        };
        let file = file.to_string_lossy();
        // A canvas of the open space is switched to, with its undo history
        // and camera as they were left.
        let open = (self.space.as_deref() == Some(folder))
            .then(|| {
                self.app
                    .space()
                    .canvases()
                    .iter()
                    .find(|canvas| canvas.file == *file)
            })
            .flatten()
            .map(|canvas| canvas.id.clone());
        if let Some(canvas) = open {
            self.dispatch(Event::Action(Action::Canvas(CanvasAction::Switch(canvas))));
        } else if let Err(error) = self.open_space(folder, Some(&file)) {
            tracing::error!("{error:#}");
        }
    }

    /// Asks for the active canvas's new name. There is no text prompt among
    /// the system dialogs, so this is the save panel with the name filled
    /// in: whatever is typed there is the name, and no file is written by
    /// the panel. It stands in until the sidebar renames in place.
    fn choose_canvas_name(&mut self) {
        let current = self.app.space().active().name.clone();
        let dialog = (self.dialog())
            .set_title("Rename canvas")
            .set_file_name(&current);
        let Some(path) = dialog.save_file() else {
            return;
        };
        let Some(name) = typed_name(&path) else {
            return;
        };
        self.dispatch(Event::Action(Action::Canvas(CanvasAction::Rename {
            canvas: None,
            name,
        })));
    }
}

/// The name typed into the rename panel: the chosen path's last part,
/// without a `.canvas` the user may have added.
fn typed_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy();
    let name = name.strip_suffix(".canvas").unwrap_or(&name).trim();
    (!name.is_empty()).then(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_typed_name_is_the_last_part_without_the_extension() {
        assert_eq!(
            typed_name(Path::new("/space/Home page")).as_deref(),
            Some("Home page")
        );
        assert_eq!(
            typed_name(Path::new("/space/Home.canvas")).as_deref(),
            Some("Home")
        );
        assert_eq!(typed_name(Path::new("/space/ .canvas")), None);
    }
}
