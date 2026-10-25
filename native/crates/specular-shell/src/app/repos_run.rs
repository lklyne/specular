//! The repos file: which repo each origin is bound to, in `repos.json` in
//! this app's data folder.

use std::ffi::OsString;
use std::path::PathBuf;

use specular_agent::Repos;
use specular_interact::{Action, Event, RepoAction};

use super::runtime::{Runtime, ShellWindow};
use crate::persist::write_atomic;
use crate::prefs;

const FILE_NAME: &str = "repos.json";

/// Answers the folder dialog without showing it, for scripted runs.
const PICK_VARIABLE: &str = "SPECULAR_REPO_PICK";

/// Where the repos file is: `SPECULAR_REPOS_FILE`, else `repos.json` in
/// this app's data folder, `data`.
fn file_in(variable: impl Fn(&str) -> Option<OsString>, data: Option<PathBuf>) -> Option<PathBuf> {
    let named = variable("SPECULAR_REPOS_FILE").filter(|value| !value.is_empty());
    named
        .map(PathBuf::from)
        .or(data.map(|folder| folder.join(FILE_NAME)))
}

pub(super) fn file() -> Option<PathBuf> {
    file_in(|name| std::env::var_os(name), prefs::folder())
}

impl<W: ShellWindow> Runtime<W> {
    /// Hands the app the repos saved by an earlier run. No file is no repos.
    pub(super) fn load_repos(&mut self) {
        let Some(path) = self.repos_file.clone() else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(text) => self.dispatch(Event::ReposLoaded(Box::new(Repos::from_json(&text)))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(path = %path.display(), "repos not read: {error}"),
        }
    }

    /// Writes the connected repos to this app's `repos.json`.
    pub(super) fn save_repos(&mut self) {
        let Some(path) = self.repos_file.as_deref() else {
            return;
        };
        let written = path
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| write_atomic(path, &self.app.repos().to_json()));
        if let Err(error) = written {
            tracing::warn!(path = %path.display(), "repos not saved: {error}");
        }
    }

    /// Asks for a folder to connect, or to bind `origin` to.
    /// `SPECULAR_REPO_PICK` answers in place of the dialog.
    pub(super) fn pick_repo_folder(&mut self, origin: Option<String>) {
        let chosen = std::env::var_os(PICK_VARIABLE)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .or_else(repo_dialog);
        let Some(folder) = chosen else {
            return;
        };
        let path = folder.to_string_lossy().into_owned();
        let action = match origin {
            Some(origin) => RepoAction::Bind { origin, path },
            None => RepoAction::Connect(path),
        };
        self.dispatch(Event::Action(Action::Repo(action)));
    }
}

fn repo_dialog() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Choose repo folder")
        .pick_folder()
}
