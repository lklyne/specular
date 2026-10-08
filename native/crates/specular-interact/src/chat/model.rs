//! [`ChatModel`]: everything the right panel shows, as data.

use specular_agent::{Role, ThreadId};
use specular_doc::AnnotationId;

use crate::Action;

/// The whole panel.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatModel {
    /// Whether the shell draws the panel at all.
    pub available: bool,
    /// Whether it is open.
    pub visible: bool,
    /// The width it is laid out at, in logical pixels.
    pub width: f32,
    /// Opens or closes it.
    pub toggle: Action,
    /// The header: the open thread's title, else `Threads`.
    pub title: String,
    /// The active canvas's threads, newest first: the list when none is
    /// open, and the switcher.
    pub threads: Vec<ThreadRow>,
    /// Starts an empty thread.
    pub new_thread: Action,
    /// Back to the list, while a thread is open.
    pub back: Option<Action>,
    /// Archives the open thread.
    pub close: Option<Action>,
    /// The open thread's conversation. `None` shows the list instead.
    pub transcript: Option<Transcript>,
    /// The field and what hangs above it.
    pub composer: Composer,
}

/// One thread in the list.
#[derive(Debug, Clone, PartialEq)]
pub struct ThreadRow {
    /// The thread.
    pub id: ThreadId,
    /// Its title.
    pub title: String,
    /// Whether it is still a draft, nothing sent.
    pub draft: bool,
    /// Whether it is the open one.
    pub active: bool,
    /// When it last changed, ISO 8601.
    pub updated_at: String,
    /// Opens it.
    pub select: Action,
    /// Archives it.
    pub close: Action,
}

/// The open thread.
#[derive(Debug, Clone, PartialEq)]
pub struct Transcript {
    /// The messages that were sent, oldest first.
    pub messages: Vec<Bubble>,
    /// What the agent has said so far this run.
    pub streaming: Option<String>,
    /// The agent at work.
    pub run: Option<RunBar>,
    /// Why the last run failed.
    pub error: Option<String>,
    /// What to say when there is nothing to show.
    pub empty_hint: Option<String>,
}

/// One sent message.
#[derive(Debug, Clone, PartialEq)]
pub struct Bubble {
    /// The message.
    pub id: String,
    /// Who wrote it.
    pub role: Role,
    /// The words.
    pub text: String,
    /// The comment it came from.
    pub annotation: Option<AnnotationId>,
    /// Whether that comment is the focused one: the renderer flashes the
    /// bubble.
    pub focused: bool,
    /// Brings the comment's pin into focus, for a bubble whose pin is still
    /// on the canvas.
    pub focus: Option<Action>,
    /// The pasted images' files: absolute when the space has a folder, else
    /// relative to it.
    pub images: Vec<String>,
}

/// The strip shown while the agent works.
#[derive(Debug, Clone, PartialEq)]
pub struct RunBar {
    /// What it is doing now.
    pub label: String,
    /// What it did, oldest first.
    pub log: Vec<String>,
    /// Stops the run.
    pub stop: Action,
}

/// The composer.
#[derive(Debug, Clone, PartialEq)]
pub struct Composer {
    /// The comment being written.
    pub draft: Option<DraftChip>,
    /// Sent comments that are still open on the canvas.
    pub open_comments: Option<OpenComments>,
    /// What waits to be sent.
    pub queued: Vec<QueuedChip>,
    /// The field's hint.
    pub placeholder: String,
    /// Whether Send does something with nothing typed: it sends the queue.
    pub can_send_empty: bool,
    /// What this turn is about.
    pub pill: PillChip,
    /// The folder the agent works in, by its last path segment.
    pub folder: Option<String>,
    /// Whether the open thread's agent is working.
    pub running: bool,
}

/// What kind of gesture made a draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftKind {
    /// A page element.
    Element,
    /// A point of the canvas.
    Point,
    /// A region.
    Region,
    /// The selection.
    Selection,
}

/// The comment draft as a removable chip.
#[derive(Debug, Clone, PartialEq)]
pub struct DraftChip {
    /// The words on it.
    pub label: String,
    /// What made it.
    pub kind: DraftKind,
    /// Drops the draft.
    pub remove: Action,
}

/// The open thread's sent comments that are still open.
#[derive(Debug, Clone, PartialEq)]
pub struct OpenComments {
    /// How many.
    pub count: usize,
    /// Resolves them all as one step.
    pub resolve: Action,
}

/// A message waiting to be sent.
#[derive(Debug, Clone, PartialEq)]
pub struct QueuedChip {
    /// Its words, or `Image` / `N images` when it has none.
    pub text: String,
    /// The comment it came from.
    pub annotation: Option<AnnotationId>,
}

/// What a pill points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillKind {
    /// A DOM node.
    Dom,
    /// The focused comment.
    Comment,
    /// Selected items.
    Selection,
    /// Nothing: the canvas.
    Canvas,
}

/// The pill above the field.
#[derive(Debug, Clone, PartialEq)]
pub struct PillChip {
    /// The words on it.
    pub label: String,
    /// What it points at.
    pub kind: PillKind,
}
