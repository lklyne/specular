//! The repos file: which repo each origin is bound to, kept in the Electron
//! app's `repos.json` so both apps agree.

use std::ffi::OsString;
use std::path::PathBuf;

use specular_agent::Repos;
use specular_interact::{Action, Event, RepoAction};

use super::runtime::{Runtime, ShellWindow};
use crate::persist::write_atomic;
use crate::space::electron_user_data;

const FILE_NAME: &str = "repos.json";

/// Answers the folder dialog without showing it, for scripted runs.
const PICK_VARIABLE: &str = "SPECULAR_REPO_PICK";

/// Where the repos file is: `SPECULAR_REPOS_FILE`, else `repos.json` in
/// `SPECULAR_NATIVE_CONFIG_DIR` (a run that must not touch the real one),
/// else in the Electron app's data folder.
fn file_in(variable: impl Fn(&str) -> Option<OsString>, macos: bool) -> Option<PathBuf> {
    let set = |name: &str| variable(name).filter(|value| !value.is_empty());
    if let Some(file) = set("SPECULAR_REPOS_FILE") {
        return Some(PathBuf::from(file));
    }
    if let Some(folder) = set("SPECULAR_NATIVE_CONFIG_DIR") {
        return Some(PathBuf::from(folder).join(FILE_NAME));
    }
    Some(electron_user_data(variable, macos)?.join(FILE_NAME))
}

pub(super) fn file() -> Option<PathBuf> {
    file_in(|name| std::env::var_os(name), cfg!(target_os = "macos"))
}

impl<W: ShellWindow> Runtime<W> {
    /// Hands the app the repos saved by an earlier run, or by the Electron
    /// app. A missing file is no repos.
    pub(super) fn load_repos(&mut self) {
        let Some(path) = self.repos_file.as_deref() else {
            return;
        };
        match std::fs::read_to_string(path) {
            Ok(text) => self.dispatch(Event::ReposLoaded(Box::new(Repos::from_json(&text)))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(path = %path.display(), "repos not read: {error}"),
        }
    }

    /// Writes the connected repos to `repos.json`.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(pairs: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs = pairs.to_vec();
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn the_repos_file_is_the_override_then_the_config_folder_then_electrons() {
        let home = ("HOME", "/Users/me");
        let cases = [
            (
                vec![
                    home,
                    ("SPECULAR_REPOS_FILE", "/x/r.json"),
                    ("SPECULAR_NATIVE_CONFIG_DIR", "/cfg"),
                ],
                Some("/x/r.json"),
            ),
            (
                vec![home, ("SPECULAR_NATIVE_CONFIG_DIR", "/cfg")],
                Some("/cfg/repos.json"),
            ),
            (
                vec![home, ("SPECULAR_REPOS_FILE", "")],
                Some("/Users/me/Library/Application Support/Specular/repos.json"),
            ),
            (vec![], None),
        ];
        for (variables, expected) in cases {
            let found = file_in(environment(&variables), true);
            assert_eq!(found, expected.map(PathBuf::from), "{variables:?}");
        }
    }
}
