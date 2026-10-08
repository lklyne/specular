//! The pure model of the canvas agent thread (the right-panel chat).
//!
//! No I/O, no clock, no randomness: the caller passes ids and ISO-8601
//! timestamps in, and turns the [`Changed`] and [`Outcome`] values that come
//! back into effects (files to write, a `claude` process to run).

mod cli;
mod describe;
mod json;
mod pill;
mod prompt;
mod reply;
mod run;
mod stream;
mod text;
mod thread;
mod threads;
mod threads_run;

pub use cli::{ALLOWED_TOOLS, Permissions, RunConfig, RunRequest, SPECULAR_CONTEXT, claude_args};
pub use json::{Index, index_json, parse_index};
pub use pill::{
    CanvasSelection, FocusedAnnotation, InspectNode, Pill, PillInput, focus_prompt, resolve,
};
pub use prompt::{CommentContext, PromptContext, WriteTarget, follow_up_prompt, thread_prompt};
pub use reply::{Reply, parse_output};
pub use run::{MAX_EVENTS, Run, RunState};
pub use stream::{Notice, Progress, ProgressKind, parse_line};
pub use thread::{Image, MediaType, Message, Role, Status, Thread, ThreadId, title_from_messages};
pub use threads::{Changed, Threads};
pub use threads_run::{Outcome, Started};
