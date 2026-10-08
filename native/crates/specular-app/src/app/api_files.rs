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

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder under the system temp dir, removed on drop.
    struct Folder(PathBuf);

    impl Folder {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("specular-api-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_file_is_found_inside_or_outside_the_space_with_an_images_size() {
        let space = Folder::new("inspect-space");
        let outside = Folder::new("inspect-outside");
        std::fs::create_dir_all(space.0.join("assets")).unwrap();
        image::RgbaImage::new(30, 20)
            .save(space.0.join("assets/inside.png"))
            .unwrap();
        image::RgbaImage::new(7, 9)
            .save(outside.0.join("away.png"))
            .unwrap();
        std::fs::write(outside.0.join("away.txt"), "x").unwrap();

        let inside = inspect("assets/inside.png", Some(&space.0)).unwrap();
        assert_eq!(inside.space_path.as_deref(), Some("assets/inside.png"));
        assert_eq!(inside.image_size, Some((30, 20)));

        let away = outside.0.join("away.png");
        let away = inspect(away.to_str().unwrap(), Some(&space.0)).unwrap();
        assert_eq!(away.space_path, None);
        assert_eq!(away.image_size, Some((7, 9)));

        let text = outside.0.join("away.txt");
        let text = inspect(text.to_str().unwrap(), Some(&space.0)).unwrap();
        assert_eq!(text.image_size, None);

        assert_eq!(inspect("assets/nothing.png", Some(&space.0)), None);
        assert_eq!(inspect("assets", Some(&space.0)), None);
    }
}
