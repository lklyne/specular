//! Files dragged onto the window. winit reports them one at a time with no
//! position, so a turn's worth is gathered and sent as one drop at the
//! pointer's last known position.

use std::path::Path;

use specular_interact::{DroppedFile, Event, is_image_file};

use super::Shell;

impl Shell {
    /// Sends the files dropped since the last turn as one event.
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
            screen: self.cursor,
        });
    }
}

/// What `update` needs to know about a dropped file: where it is, whether
/// that is inside the space folder, and how large an image it is.
fn dropped_file(path: &Path, space: Option<&Path>) -> DroppedFile {
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
    }

    #[test]
    fn a_file_anywhere_else_is_outside() {
        let space = Path::new("/Users/me/Space");
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
