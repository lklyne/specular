//! Files dragged onto the window. winit reports them one at a time with no
//! position, so a turn's worth is gathered and sent as one drop at the
//! pointer's last known position.

use std::path::{Path, PathBuf};

use glam::Vec2;

use specular_interact::{DroppedFile, Event, is_image_file};

use super::runtime::{Runtime, ShellWindow};

impl<W: ShellWindow> Runtime<W> {
    /// Sends the files dropped since the last turn as one event.
    /// Files dragged onto the window at `screen`, in logical pixels of the
    /// viewport. They reach the app on the next turn, as one drop.
    pub fn drop_files(&mut self, paths: impl IntoIterator<Item = PathBuf>, screen: Option<Vec2>) {
        self.dropped.extend(paths);
        self.dropped_at = screen;
    }

    pub(super) fn flush_drops(&mut self) {
        if self.dropped.is_empty() {
            return;
        }
        let space = self.space.as_deref();
        let files = (self.dropped.drain(..))
            .map(|path| dropped_file(&path, space))
            .collect();
        self.dispatch(Event::FilesDropped {
            files,
            screen: self.dropped_at,
        });
    }
}

/// What `update` needs to know about a dropped file: where it is, whether
/// that is inside the space folder, and how large an image it is.
pub(super) fn dropped_file(path: &Path, space: Option<&Path>) -> DroppedFile {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    let text = path.to_string_lossy().into_owned();
    DroppedFile {
        space_path: space.and_then(|space| space_path(&path, space)),
        // Only the header is read.
        image_size: (is_image_file(&text))
            .then(|| image::image_dimensions(&path).ok())
            .flatten(),
        path: text,
    }
}

/// `path` relative to `space` with `/` between its parts, when it is inside.
fn space_path(path: &Path, space: &Path) -> Option<String> {
    let inside = path.strip_prefix(space).ok()?;
    let parts: Option<Vec<&str>> = (inside.components())
        .map(|part| part.as_os_str().to_str())
        .collect();
    let parts = parts.filter(|parts| !parts.is_empty())?;
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_under_the_space_folder_is_named_relative_to_it() {
        let space = Path::new("/Users/me/Space");
        assert_eq!(
            space_path(Path::new("/Users/me/Space/assets/shot.png"), space).as_deref(),
            Some("assets/shot.png")
        );
        assert_eq!(
            space_path(Path::new("/Users/me/Space/Plan.md"), space).as_deref(),
            Some("Plan.md")
        );
        assert_eq!(
            space_path(Path::new("/Users/me/Desktop/shot.png"), space),
            None
        );
        assert_eq!(
            space_path(Path::new("/Users/me/Space2/shot.png"), space),
            None
        );
        assert_eq!(space_path(space, space), None);
    }

    #[test]
    fn only_the_image_types_the_canvas_draws_are_sized() {
        use image::ImageFormat;

        let dir = std::env::temp_dir().join(format!("specular-drop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut bytes = Vec::new();
        image::DynamicImage::new_rgba8(8, 4)
            .write_to(&mut std::io::Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        std::fs::write(dir.join("shot.png"), &bytes).unwrap();
        // A type the decoder reads but the canvas does not draw as an image.
        std::fs::write(dir.join("not-listed.apng"), &bytes).unwrap();
        let sized = dropped_file(&dir.join("shot.png"), None);
        let note = dropped_file(&dir.join("not-listed.apng"), None);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(sized.image_size, Some((8, 4)));
        assert_eq!(note.image_size, None);
    }

    #[test]
    fn a_dropped_file_that_is_not_an_image_has_no_size() {
        let file = dropped_file(Path::new("/nowhere/Plan.md"), Some(Path::new("/nowhere")));
        assert_eq!(
            file,
            DroppedFile {
                path: "/nowhere/Plan.md".to_owned(),
                space_path: Some("Plan.md".to_owned()),
                image_size: None,
            }
        );
    }
}
