//! Asking for a folder (a space to open or make, a repo to connect), the
//! folder that comes back, and showing the space in the file manager.
//!
//! Every folder dialog takes this one path. The dialog is the window's to
//! show: a modal one opened from inside an event would run the window's
//! callbacks under the event still being handled. So the effect leaves a
//! note here, the window takes it on its next turn with
//! [`Runtime::take_folder_dialog`], and the answer comes back through
//! [`Runtime::folder_chosen`].

use std::path::{Path, PathBuf};

use specular_interact::{Action, Event, RepoAction};

use super::runtime::{Runtime, ShellWindow};

/// A folder the app asked the user for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderAsk {
    /// A space to open, or to make when `create`, which only words the
    /// dialog.
    Space {
        /// Whether the dialog is worded for making a space.
        create: bool,
    },
    /// A repo to connect, or to bind `origin` to.
    Repo {
        /// The origin the repo is bound to, if one is named.
        origin: Option<String>,
    },
}

impl FolderAsk {
    /// The words on the dialog's button.
    pub fn prompt(&self) -> &'static str {
        match self {
            Self::Space { create: true } => "Create space",
            Self::Space { create: false } => "Open space",
            Self::Repo { .. } => "Choose repo folder",
        }
    }

    /// The variable that answers in place of the dialog, for scripted runs.
    const fn variable(&self) -> &'static str {
        match self {
            Self::Space { .. } => "SPECULAR_SPACE_PICK",
            Self::Repo { .. } => "SPECULAR_REPO_PICK",
        }
    }
}

impl<W: ShellWindow> Runtime<W> {
    /// The app asked for a folder. The ask's variable, when set, answers in
    /// place of the dialog.
    pub(super) fn ask_for_folder(&mut self, ask: FolderAsk) {
        let scripted = std::env::var_os(ask.variable()).filter(|path| !path.is_empty());
        match scripted {
            Some(folder) => self.folder_chosen(ask, &PathBuf::from(folder)),
            None => self.folder_dialog = Some(ask),
        }
    }

    /// The folder dialog the app is waiting on, once. The window shows it
    /// and answers with [`folder_chosen`](Self::folder_chosen), or with
    /// nothing when it is cancelled.
    pub fn take_folder_dialog(&mut self) -> Option<FolderAsk> {
        self.folder_dialog.take()
    }

    /// The user chose `folder` for `ask`.
    pub fn folder_chosen(&mut self, ask: FolderAsk, folder: &Path) {
        match ask {
            FolderAsk::Space { .. } => self.choose_space(folder),
            FolderAsk::Repo { origin } => {
                let path = folder.to_string_lossy().into_owned();
                let action = match origin {
                    Some(origin) => RepoAction::Bind { origin, path },
                    None => RepoAction::Connect(path),
                };
                self.dispatch(Event::Action(Action::Repo(action)));
            }
        }
    }

    /// Opens `folder` as the space and remembers it for the next launch.
    /// The space that was open is saved first and otherwise left alone. A
    /// folder that cannot be opened changes nothing.
    pub(crate) fn choose_space(&mut self, folder: &Path) {
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
