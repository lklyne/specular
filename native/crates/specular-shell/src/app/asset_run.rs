//! The asset effects: putting a pasted or dropped file into the space
//! folder, where the file entity `update` made expects it.

use std::io;
use std::path::{Component, Path, PathBuf};

use super::runtime::{Runtime, ShellWindow};

impl<W: ShellWindow> Runtime<W> {
    pub(super) fn write_asset(&self, file: &str, bytes: &[u8]) {
        let written =
            destination(self.space.as_deref(), file).and_then(|path| std::fs::write(path, bytes));
        if let Err(error) = written {
            tracing::warn!(file, "the pasted file was not written: {error}");
        }
    }

    pub(super) fn copy_asset(&self, from: &str, file: &str) {
        let copied = destination(self.space.as_deref(), file)
            .and_then(|path| std::fs::copy(from, path).map(drop));
        if let Err(error) = copied {
            tracing::warn!(from, file, "the dropped file was not copied: {error}");
        }
    }
}

/// Where `file` goes inside `space`, with its folder made. A canvas that is
/// not in a file has no space folder, and a path that would leave the folder
/// is refused.
fn destination(space: Option<&Path>, file: &str) -> io::Result<PathBuf> {
    let space = space.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "the canvas is not in a file, so it has no space folder",
        )
    })?;
    let relative = Path::new(file);
    let stays_inside = (relative.components()).all(|part| matches!(part, Component::Normal(_)));
    if !stays_inside {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the path is not inside the space folder",
        ));
    }
    let path = space.join(relative);
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    Ok(path)
}
