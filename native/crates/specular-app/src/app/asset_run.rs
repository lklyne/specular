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

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder under the system temp dir, removed on drop.
    struct Space(PathBuf);

    impl Space {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("specular-assets-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Space {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn an_asset_goes_under_the_space_folder_and_its_folder_is_made() {
        let space = Space::new("destination");
        let path = destination(Some(&space.0), "assets/shot.png").unwrap();
        assert_eq!(path, space.0.join("assets").join("shot.png"));
        assert!(space.0.join("assets").is_dir());
    }

    #[test]
    fn a_path_that_leaves_the_space_folder_is_refused() {
        let space = Space::new("escape");
        for file in ["../shot.png", "/tmp/shot.png", "assets/../../shot.png"] {
            assert!(destination(Some(&space.0), file).is_err(), "{file}");
        }
        assert!(destination(None, "assets/shot.png").is_err());
    }
}
