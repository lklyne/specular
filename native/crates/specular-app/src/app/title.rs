//! The window title: the canvas's name, and whether it has changes that are
//! not in its file yet.

use std::path::Path;

use super::Shell;
use crate::persist::Persistence;

/// The title of a window showing a canvas that is not in a file.
const APP_NAME: &str = "Specular";

impl Shell {
    /// Brings the title in step with the open file and its unsaved changes.
    pub(super) fn refresh_title(&mut self) {
        // A benchmark window keeps the title it opened with.
        if self.bench.is_some() {
            return;
        }
        let unsaved = (self.persist.as_ref()).is_some_and(Persistence::has_unsaved);
        let title = window_title(self.options.canvas.as_deref(), unsaved);
        if title == self.title {
            return;
        }
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.set_title(&title);
            #[cfg(target_os = "macos")]
            winit::platform::macos::WindowExtMacOS::set_document_edited(&*gpu.window, unsaved);
            self.title = title;
        }
    }
}

/// The canvas's file name without `.canvas`, marked while it has unsaved
/// changes.
fn window_title(canvas: Option<&Path>, unsaved: bool) -> String {
    let name = canvas
        .and_then(Path::file_stem)
        .map(|stem| stem.to_string_lossy());
    match (name, unsaved) {
        (Some(name), true) => format!("{name} — Edited"),
        (Some(name), false) => name.into_owned(),
        (None, _) => APP_NAME.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_is_the_canvas_name_without_its_extension() {
        let canvas = Path::new("/Users/me/Space/Home page.canvas");
        assert_eq!(window_title(Some(canvas), false), "Home page");
    }

    #[test]
    fn unsaved_changes_are_marked() {
        let canvas = Path::new("Welcome.canvas");
        assert_eq!(window_title(Some(canvas), true), "Welcome — Edited");
    }

    #[test]
    fn a_canvas_with_no_file_is_named_after_the_app() {
        assert_eq!(window_title(None, false), "Specular");
        assert_eq!(window_title(None, true), "Specular");
    }
}
