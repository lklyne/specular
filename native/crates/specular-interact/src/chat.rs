//! The right panel's chat: the canvas agent thread, as state, actions and a
//! model.
//!
//! `specular-agent` holds the pure rules (threads, prompts, the claude
//! stream). This module owns them on the [`App`]: the space's
//! [`Threads`] live beside the [`Space`](crate::Space), the panel's own
//! state (available, shown, width) is in the session, and every change comes
//! through [`update`](crate::update) as an [`Event`](crate::Event) or an
//! [`Action::Chat`](crate::Action::Chat). Files and the `claude` process are
//! [`Effect`](crate::Effect)s. [`chat`] reads the whole panel as data, as
//! [`sidebar`](crate::sidebar()) reads the sidebar.
//!
//! Comments and chat are one conversation: committing a comment queues it
//! into a thread and writes the thread's id on the annotation in the same
//! undo step.

mod action;
mod build;
mod comments;
mod commit;
mod draft;
mod effects;
mod model;
mod notice;
mod pill;
mod prompt;
mod send;

use specular_agent::Threads;

pub(crate) use self::action::run;
pub use self::action::{ChatAction, ImageUpload};
pub use self::build::chat;
pub(crate) use self::commit::comment as commit_comment;
pub use self::model::{
    AutoChip, Bubble, ChatModel, Composer, DraftChip, DraftKind, OpenComments, PillChip, PillKind,
    QueuedChip, RunBar, ThreadRow, Transcript,
};
pub(crate) use self::notice::{on_agent, on_loaded, open_thread_of};
use crate::App;

/// The width the panel starts at: `DEVTOOLS_DEFAULT_WIDTH`.
pub const CHAT_WIDTH: f32 = 400.0;
/// The narrowest the panel gets: `DEVTOOLS_MIN_WIDTH`.
pub const CHAT_MIN_WIDTH: f32 = 280.0;
/// The widest the panel gets: `DEVTOOLS_MAX_WIDTH`.
pub const CHAT_MAX_WIDTH: f32 = 960.0;

/// The panel's own state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChatView {
    available: bool,
    shown: bool,
    width: f32,
}

impl Default for ChatView {
    fn default() -> Self {
        Self {
            available: false,
            shown: false,
            width: CHAT_WIDTH,
        }
    }
}

impl ChatView {
    /// Whether the shell draws a right panel at all. Without one, a comment
    /// is written in the card on the canvas.
    pub fn available(&self) -> bool {
        self.available
    }

    /// Whether the panel is open. It starts closed, and opens when a comment
    /// draft needs its field.
    pub fn shown(&self) -> bool {
        self.available && self.shown
    }

    /// The width the panel is laid out at, in logical pixels.
    pub fn width(&self) -> f32 {
        self.width
    }

    pub(crate) fn set_available(&mut self, available: bool) {
        self.available = available;
    }

    pub(crate) fn show(&mut self) {
        self.shown = true;
    }

    pub(crate) fn toggle(&mut self) {
        self.shown = !self.shown;
    }

    pub(crate) fn resize(&mut self, width: f32) {
        if width.is_finite() {
            self.width = width.clamp(CHAT_MIN_WIDTH, CHAT_MAX_WIDTH);
        }
    }
}

impl App {
    /// Every thread of the space, the active thread of each canvas, and the
    /// runs in flight.
    pub fn threads(&self) -> &Threads {
        &self.threads
    }

    /// The right panel's state.
    pub fn chat_view(&self) -> &ChatView {
        &self.session.chat
    }

    /// The text of `.specular/threads/index.json`: the active canvas's open
    /// thread, and the open thread of every canvas.
    pub fn thread_index_json(&self) -> String {
        self.threads
            .index_json(Some(self.space.active().id.as_str()))
    }
}
