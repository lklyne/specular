//! The window title: the canvas's name, and whether it has changes that are
//! not in its file yet.

use super::Shell;

/// The title of a window showing a canvas that is not in a file.
const APP_NAME: &str = "Specular";

impl Shell {
    /// Brings the title in step with the open file and its unsaved changes.
    pub(super) fn refresh_title(&mut self) {
        // A benchmark window keeps the title it opened with.
        if self.bench.is_some() {
            return;
        }
        let unsaved = self.active_unsaved();
        let name = (self.files.as_ref()).map(|_| self.app.space().active().name.as_str());
        let title = window_title(name, unsaved);
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

/// The active canvas's name, marked while it has unsaved changes.
fn window_title(canvas: Option<&str>, unsaved: bool) -> String {
    match (canvas, unsaved) {
        (Some(name), true) => format!("{name} — Edited"),
        (Some(name), false) => name.to_owned(),
        (None, _) => APP_NAME.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_is_the_canvas_name() {
        assert_eq!(window_title(Some("Home page"), false), "Home page");
    }

    #[test]
    fn unsaved_changes_are_marked() {
        assert_eq!(window_title(Some("Welcome"), true), "Welcome — Edited");
    }

    #[test]
    fn a_canvas_with_no_file_is_named_after_the_app() {
        assert_eq!(window_title(None, false), "Specular");
        assert_eq!(window_title(None, true), "Specular");
    }
}
