//! The window title: the canvas's name, and whether it has changes that are
//! not in its file yet.

use super::runtime::{Runtime, ShellWindow};

/// The title of a window showing a canvas that is not in a file.
const APP_NAME: &str = "Specular";

impl<W: ShellWindow> Runtime<W> {
    /// Brings the title in step with the open file and its unsaved changes.
    pub fn refresh_title(&mut self) {
        let unsaved = self.active_unsaved();
        let name = (self.files.as_ref()).map(|_| self.app.space().active().name.as_str());
        let scratch = self.scratch.is_some() && self.scratch == self.space;
        let title = window_title(name, scratch, unsaved);
        if title == self.title {
            return;
        }
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.set_title(&title, unsaved);
            self.title = title;
        }
    }
}

/// The active canvas's name, marked while it is in the scratch space and
/// while it has unsaved changes.
fn window_title(canvas: Option<&str>, scratch: bool, unsaved: bool) -> String {
    let Some(name) = canvas else {
        return APP_NAME.to_owned();
    };
    let space = if scratch { " (scratch space)" } else { "" };
    let edited = if unsaved { " — Edited" } else { "" };
    format!("{name}{space}{edited}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_title_names_the_canvas_scratch_space_and_unsaved_changes() {
        for (name, scratch, edited, expected) in [
            (Some("Home page"), false, false, "Home page"),
            (Some("Welcome"), false, true, "Welcome — Edited"),
            (Some("Welcome"), true, false, "Welcome (scratch space)"),
            (
                Some("Welcome"),
                true,
                true,
                "Welcome (scratch space) — Edited",
            ),
            (None, false, false, "Specular"),
            (None, true, true, "Specular"),
        ] {
            assert_eq!(window_title(name, scratch, edited), expected);
        }
    }
}
