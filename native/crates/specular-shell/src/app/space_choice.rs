//! Choosing the space: the folder dialog the app asks for, the folder that
//! comes back, and showing it in the file manager.
//!
//! The dialog is the shell's to show, as File > Open space… is: each
//! toolkit has its own, and a modal one opened from inside an event would
//! run the window's callbacks under the event still being handled. So the
//! effect leaves a note here, the shell takes it on its next turn, and the
//! answer comes back through [`Runtime::choose_space`].

use std::path::{Path, PathBuf};

use super::runtime::{Runtime, ShellWindow};

/// Answers the folder dialog without showing it, for scripted runs.
const PICK_VARIABLE: &str = "SPECULAR_SPACE_PICK";

impl<W: ShellWindow> Runtime<W> {
    /// The app asked for a space folder. `SPECULAR_SPACE_PICK` answers in
    /// place of the dialog.
    pub(super) fn ask_for_space(&mut self, create: bool) {
        let scripted = std::env::var_os(PICK_VARIABLE).filter(|path| !path.is_empty());
        match scripted {
            Some(folder) => self.choose_space(&PathBuf::from(folder)),
            None => self.space_dialog = Some(create),
        }
    }

    /// The folder dialog the app is waiting on, once: whether it is worded
    /// for making a space. The shell shows it and answers with
    /// [`choose_space`](Self::choose_space), or with nothing when it is
    /// cancelled.
    pub fn take_space_dialog(&mut self) -> Option<bool> {
        self.space_dialog.take()
    }

    /// Opens `folder` as the space and remembers it for the next launch.
    /// The space that was open is saved first and otherwise left alone. A
    /// folder that cannot be opened changes nothing.
    pub fn choose_space(&mut self, folder: &Path) {
        match self.open_space(folder, None) {
            Ok(()) => self.remember_space(folder),
            Err(error) => tracing::error!("{error:#}"),
        }
    }

    /// Shows the open space's folder in the file manager.
    pub(super) fn reveal_space(&self) {
        let Some(folder) = self.space.as_deref() else {
            return;
        };
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        if let Err(error) = std::process::Command::new(opener).arg(folder).spawn() {
            tracing::warn!(folder = %folder.display(), "the space folder was not shown: {error}");
        }
    }
}
