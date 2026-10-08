//! The right panel in a test: turning it on, sending from its composer,
//! playing the agent's side, and the model as text for `insta`.

use std::fmt::Write as _;

use specular_interact::{
    Action, ChatAction, ChatModel, Event, ImageUpload, Notice, ThreadId, ThreadRole, chat,
};

use crate::TestApp;

impl TestApp {
    /// Tells the app the shell draws a right panel, as the GPUI shell does
    /// at startup. A comment draft then finishes in the panel, not on the
    /// canvas.
    pub fn with_chat_panel(&mut self) -> &mut Self {
        self.send(Event::ChatPanel(true))
    }

    /// The right panel as it is now.
    pub fn chat(&self) -> ChatModel {
        chat(self.app())
    }

    /// Presses Send with `text` typed in the composer.
    pub fn send_chat(&mut self, text: &str) -> &mut Self {
        self.send_chat_with(text, Vec::new())
    }

    /// Presses Send with `text` typed and `images` pasted.
    pub fn send_chat_with(&mut self, text: &str, images: Vec<ImageUpload>) -> &mut Self {
        self.act(Action::Chat(ChatAction::Send {
            text: text.to_owned(),
            images,
        }))
    }

    /// The agent running the open thread says `notice`, as the shell relays
    /// it.
    #[track_caller]
    pub fn agent_says(&mut self, notice: Notice) -> &mut Self {
        let Some(thread) = self.chat_thread_id() else {
            panic!("no thread is open for the agent to talk in");
        };
        self.agent_says_in(&thread, notice)
    }

    /// The agent running `thread` says `notice`.
    pub fn agent_says_in(&mut self, thread: &ThreadId, notice: Notice) -> &mut Self {
        self.send(Event::Agent {
            thread: thread.clone(),
            notice,
        })
    }

    /// The open thread's id.
    pub fn chat_thread_id(&self) -> Option<ThreadId> {
        let tab = self.app().space().active().id.as_str();
        self.app()
            .threads()
            .active(tab)
            .map(|thread| thread.id.clone())
    }

    /// The panel as stable text. See [`chat_snapshot`].
    pub fn chat_snapshot(&self) -> String {
        chat_snapshot(&self.chat())
    }
}

fn yes_no(on: bool) -> &'static str {
    if on { "yes" } else { "no" }
}

/// The panel as stable text: the header, the list, the transcript and the
/// composer, a line each. Ids and actions are left out; a test reads them
/// from the model.
pub fn chat_snapshot(model: &ChatModel) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "chat available={} visible={} width={} title={:?}",
        yes_no(model.available),
        yes_no(model.visible),
        model.width,
        model.title
    );
    for row in &model.threads {
        let _ = writeln!(
            out,
            "thread {} {:?}{} {}",
            if row.active { "[x]" } else { "[ ]" },
            row.title,
            if row.draft { " draft" } else { "" },
            row.updated_at
        );
    }
    let _ = writeln!(
        out,
        "controls back={} close={}",
        yes_no(model.back.is_some()),
        yes_no(model.close.is_some())
    );
    if let Some(transcript) = &model.transcript {
        for bubble in &transcript.messages {
            let role = match bubble.role {
                ThreadRole::User => "user",
                ThreadRole::Agent => "agent",
            };
            let pin = bubble
                .annotation
                .as_ref()
                .map_or_else(String::new, |id| format!(" pin={id}"));
            let focus = if bubble.focused { " focused" } else { "" };
            let live = if bubble.focus.is_some() { " live" } else { "" };
            let _ = writeln!(out, "bubble {role} {:?}{pin}{focus}{live}", bubble.text);
            for image in &bubble.images {
                let _ = writeln!(out, "  image {image}");
            }
        }
        if let Some(text) = &transcript.streaming {
            let _ = writeln!(out, "streaming {text:?}");
        }
        if let Some(run) = &transcript.run {
            let _ = writeln!(out, "run {:?}", run.label);
            for line in &run.log {
                let _ = writeln!(out, "  log {line:?}");
            }
        }
        if let Some(error) = &transcript.error {
            let _ = writeln!(out, "error {error:?}");
        }
        if let Some(hint) = &transcript.empty_hint {
            let _ = writeln!(out, "hint {hint:?}");
        }
    } else {
        out.push_str("list\n");
    }
    let composer = &model.composer;
    if let Some(draft) = &composer.draft {
        let _ = writeln!(out, "draft {:?} {:?}", draft.kind, draft.label);
    }
    if let Some(open) = &composer.open_comments {
        let _ = writeln!(out, "open-comments {}", open.count);
    }
    for chip in &composer.queued {
        let pin = chip
            .annotation
            .as_ref()
            .map_or_else(String::new, |id| format!(" pin={id}"));
        let _ = writeln!(out, "queued {:?}{pin}", chip.text);
    }
    let _ = writeln!(
        out,
        "composer placeholder={:?} send-empty={} running={} pill={:?} {:?} folder={:?}",
        composer.placeholder,
        yes_no(composer.can_send_empty),
        yes_no(composer.running),
        composer.pill.kind,
        composer.pill.label,
        composer.folder
    );
    out.trim_end().to_owned()
}

/// Asserts the right panel of a [`TestApp`] against an inline snapshot.
#[macro_export]
macro_rules! assert_chat_snapshot {
    ($app:expr, $($rest:tt)*) => {
        $crate::insta::assert_snapshot!($app.chat_snapshot(), $($rest)*)
    };
}
