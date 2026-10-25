//! What the API's `file` and Document creates need to know about the disk:
//! the facts of a file named by path, and the names the space folder holds.

use std::path::{Path, PathBuf};

use specular_interact::DroppedFile;

use super::drop_run::dropped_file;
use super::runtime::{Runtime, ShellWindow};

impl<W: ShellWindow> Runtime<W> {
    /// The file at `path`, a relative one being read from the space folder:
    /// where it is, whether that is inside the folder, and an image's size.
    /// `None` when it is not a file.
    pub(super) fn inspect_space_file(&self, path: &str) -> Option<DroppedFile> {
        inspect(path, self.space.as_deref())
    }

    /// The names directly in the space folder.
    pub(super) fn list_space_folder(&self) -> Vec<String> {
        let Some(folder) = self.space.as_deref() else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(folder) else {
            return Vec::new();
        };
        (entries.filter_map(Result::ok))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect()
    }
}

/// Both paths are resolved through links first, so a space folder reached
/// through a symlink (as the system temp folder is) still contains its files.
fn inspect(path: &str, space: Option<&Path>) -> Option<DroppedFile> {
    let space = space.map(|space| space.canonicalize().unwrap_or_else(|_| space.to_owned()));
    let given = PathBuf::from(path);
    let full = match &space {
        Some(space) if given.is_relative() => space.join(&given),
        Some(_) | None => given,
    };
    let full = full.canonicalize().ok().filter(|full| full.is_file())?;
    Some(dropped_file(&full, space.as_deref()))
}
