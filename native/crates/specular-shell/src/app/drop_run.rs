//! Files dragged onto the window, gathered over a turn and sent as one
//! drop.

use std::path::{Path, PathBuf};

use glam::Vec2;

use specular_interact::{DroppedFile, Event, is_image_file};

use super::runtime::{Runtime, ShellWindow};

impl<W: ShellWindow> Runtime<W> {
    /// Files dragged onto the window at `screen`, in logical pixels of the
    /// viewport. They reach the app on the next turn, as one drop.
    pub fn drop_files(&mut self, paths: impl IntoIterator<Item = PathBuf>, screen: Option<Vec2>) {
        self.dropped.extend(paths);
        self.dropped_at = screen;
    }

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
            screen: self.dropped_at,
        });
    }
}

/// What `update` needs to know about a dropped file: where it is, whether
/// that is inside the space folder, and how large an image it is.
pub(crate) fn dropped_file(path: &Path, space: Option<&Path>) -> DroppedFile {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_owned());
    let text = path.to_string_lossy().into_owned();
    DroppedFile {
        space_path: space.and_then(|space| space_path(&path, space)),
        // Only the header is read.
        image_size: (is_image_file(&text))
            .then(|| crate::images::dimensions(&path))
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
