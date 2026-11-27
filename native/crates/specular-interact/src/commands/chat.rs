//! The agent threads by name (ADR 0059), so a mod can draw the right panel
//! again from the `panels/chat` key, or show any thread on its own from
//! `chat/thread/<id>`, and act on it.

use serde_json::Value;
use specular_agent::ThreadId;

use super::args::{act, always, maybe_text, text};
use super::{CommandSpec, Run, RunError};
use crate::{Action, ChatAction};

fn chat(action: ChatAction) -> Result<Run, RunError> {
    act(Action::Chat(action))
}

fn thread(args: &Value) -> Option<ThreadId> {
    maybe_text(args, "id").map(ThreadId)
}

/// The thread `key` names. A thread's id is its file's name, so a new one
/// is letters, digits, `_` and `-`, at most 64 of them.
fn thread_named(args: &Value, key: &str) -> Result<Option<ThreadId>, RunError> {
    let Some(id) = maybe_text(args, key) else {
        return Ok(None);
    };
    let fits = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    if id.is_empty() || id.len() > 64 || !id.chars().all(fits) {
        return Err(RunError::Args(format!(
            "`{key}` is letters, digits, `_` and `-`, at most 64"
        )));
    }
    Ok(Some(ThreadId(id)))
}

/// The agent threads of the right panel.
pub(super) const CHAT: &[CommandSpec] = &[
    CommandSpec {
        name: "chat.toggle",
        label: "Toggle Agent Panel",
        args: "",
        about: "Show the right panel if it is hidden, hide it if shown.",
        enabled: always,
        build: |_| chat(ChatAction::Toggle),
    },
    CommandSpec {
        name: "chat.new-thread",
        label: "New Agent Thread",
        args: "{id?: thread id, open?: bool}",
        about: "Start a thread on the canvas showing, named `id` (a fresh id without one) and opened in the panel unless `open` is false. A mod that shows a thread of its own picks the id, so it knows it at once.",
        enabled: always,
        build: |args| {
            chat(ChatAction::NewThread {
                id: thread_named(args, "id")?,
                open: args.get("open").and_then(Value::as_bool).unwrap_or(true),
            })
        },
    },
    CommandSpec {
        name: "chat.select",
        label: "",
        args: "{id: thread id}",
        about: "Show a thread of `panels/chat`.",
        enabled: always,
        build: |args| chat(ChatAction::SelectThread(ThreadId(text(args, "id")?))),
    },
    CommandSpec {
        name: "chat.back",
        label: "",
        args: "",
        about: "Go back from a thread to the list.",
        enabled: always,
        build: |_| chat(ChatAction::Back),
    },
    CommandSpec {
        name: "chat.close",
        label: "",
        args: "{id?: thread id}",
        about: "Close the thread named, or the one showing.",
        enabled: always,
        build: |args| chat(ChatAction::CloseThread(thread(args))),
    },
    CommandSpec {
        name: "chat.send",
        label: "",
        args: "{text: message, thread?: thread id}",
        about: "Send a message to the thread named, or to the thread showing, or to a new one.",
        enabled: always,
        build: |args| {
            chat(ChatAction::Send {
                text: text(args, "text")?,
                images: Vec::new(),
                thread: thread_named(args, "thread")?,
            })
        },
    },
    CommandSpec {
        name: "chat.stop",
        label: "",
        args: "{thread?: thread id}",
        about: "Stop the agent's run on the thread named, or the thread showing.",
        enabled: always,
        build: |args| chat(ChatAction::Stop(thread_named(args, "thread")?)),
    },
    CommandSpec {
        name: "chat.model",
        label: "",
        args: "{name: model name}",
        about: "Choose the model new runs use, by a name `panels/chat` lists.",
        enabled: always,
        build: |args| chat(ChatAction::SetModel(text(args, "name")?)),
    },
];
