//! What a scripted run in a window needs of the runtime that a person at
//! the window does not: a clipboard that is not the system's, and the text
//! of the canvas without waiting for an autosave.

use anyhow::Context as _;

use super::runtime::{Runtime, ShellWindow};
use crate::persist::canvas_text;

/// The clipboard of a scripted run: the text last copied, if any was.
#[derive(Debug, Clone, Default)]
pub(crate) struct HeldClipboard(pub(crate) Option<String>);

impl<W: ShellWindow> Runtime<W> {
    /// Keeps the clipboard in memory from now on, holding `text`. A copy or
    /// a cut no longer writes the system clipboard and a paste no longer
    /// reads it, so a script cannot paste what the person at the machine
    /// last copied, or replace it.
    pub fn script_clipboard(&mut self, text: Option<String>) {
        self.scripted_clipboard = Some(HeldClipboard(text));
    }

    /// Whether the clipboard is kept in memory.
    pub fn clipboard_is_scripted(&self) -> bool {
        self.scripted_clipboard.is_some()
    }

    /// The `.canvas` text an autosave of the canvas showing would write now.
    pub fn canvas_text(&self) -> anyhow::Result<String> {
        let app = self.app();
        canvas_text(&app.document_to_save(), app.canvas_camera())
            .context("writing the canvas as text")
    }
}
