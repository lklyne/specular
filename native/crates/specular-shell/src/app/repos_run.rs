//! The repos file: which repo each origin is bound to, in `repos.json` in
//! this app's data folder.

use std::ffi::OsString;
use std::path::PathBuf;

use specular_agent::Repos;
use specular_interact::Event;

use super::runtime::{Runtime, ShellWindow};
use crate::persist::write_atomic;
use crate::prefs;

const FILE_NAME: &str = "repos.json";

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
}
