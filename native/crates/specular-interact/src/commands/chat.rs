//! The right panel's agent threads by name (ADR 0059), so a mod can draw
//! the panel again from the `panels/chat` key and act on it.

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
        args: "",
        about: "Start a thread on the canvas showing.",
        enabled: always,
        build: |_| chat(ChatAction::NewThread),
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
        args: "{text: message}",
        about: "Send a message to the thread showing, or to a new one.",
        enabled: always,
        build: |args| {
            chat(ChatAction::Send {
                text: text(args, "text")?,
                images: Vec::new(),
            })
        },
    },
    CommandSpec {
        name: "chat.stop",
        label: "",
        args: "",
        about: "Stop the agent's run on the thread showing.",
        enabled: always,
        build: |_| chat(ChatAction::Stop),
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
