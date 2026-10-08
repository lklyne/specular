//! Builders the store tests share.

#![expect(clippy::expect_used, reason = "a test fails by panicking")]

use specular_agent::{Pill, PromptContext, ThreadId, Threads, WriteTarget};

pub(crate) const TAB: &str = "tab_a";

pub(crate) fn id(value: &str) -> ThreadId {
    ThreadId(value.into())
}

pub(crate) fn ctx() -> PromptContext {
    PromptContext {
        space_path: "/space".into(),
        write_target: WriteTarget::Space,
        pill: Pill::Empty,
        canvas_name: "Home".into(),
        comments: Vec::new(),
    }
}

/// A comment on pin `pin` with text `text`, queued into the canvas's thread.
pub(crate) fn comment(
    threads: &mut Threads,
    new_thread: &str,
    message: &str,
    pin: &str,
    text: &str,
) -> ThreadId {
    threads
        .queue_comment(
            TAB,
            id(new_thread),
            message,
            pin,
            text,
            "2026-01-01T00:00:01Z",
        )
        .0
}

pub(crate) fn send(threads: &mut Threads, text: &str, new_thread: &str, message: &str) -> ThreadId {
    threads
        .queue_message(
            TAB,
            id(new_thread),
            message,
            text,
            Vec::new(),
            "2026-01-01T00:00:02Z",
        )
        .expect("a thread")
        .0
}
