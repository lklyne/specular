//! The right panel's model cut into what each of its views draws, so a
//! view is drawn again only when its own part changed: a streamed word is
//! the conversation's, the label of the step the agent is on is the run
//! bar's, and neither is the header's or the composer's.

use std::rc::Rc;

use specular_interact::{Action, Bubble, ChatModel, Composer, RunBar, ThreadId, ThreadRow};

use crate::models::{keep, keep_some};

/// What the header draws.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct HeaderPart {
    pub(super) title: String,
    /// Whether a thread is open: the title is then the thread switcher.
    pub(super) open: bool,
    /// The canvas's threads, for the switcher's menu.
    pub(super) threads: Vec<ThreadRow>,
    pub(super) new_thread: Action,
    pub(super) back: Option<Action>,
    pub(super) close: Option<Action>,
}

/// What the panel's body draws.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum BodyPart {
    /// No thread is open: the canvas's threads.
    Threads(Vec<ThreadRow>),
    /// The open thread's conversation.
    Conversation(Conversation),
}

/// The open thread, without the run bar.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Conversation {
    pub(super) thread: Option<ThreadId>,
    pub(super) messages: Vec<Bubble>,
    /// What the agent has said so far this run.
    pub(super) streaming: Option<String>,
    pub(super) error: Option<String>,
    pub(super) empty_hint: Option<String>,
}

/// The panel's parts, each the same allocation for as long as it says the
/// same.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ChatParts {
    pub(super) header: Rc<HeaderPart>,
    pub(super) body: Rc<BodyPart>,
    /// The agent at work on the open thread.
    pub(super) run: Option<Rc<RunBar>>,
    pub(super) composer: Rc<Composer>,
}

fn header_of(model: &ChatModel) -> HeaderPart {
    HeaderPart {
        title: model.title.clone(),
        open: model.transcript.is_some(),
        threads: model.threads.clone(),
        new_thread: model.new_thread.clone(),
        back: model.back.clone(),
        close: model.close.clone(),
    }
}

fn body_of(model: &ChatModel) -> BodyPart {
    let Some(transcript) = &model.transcript else {
        return BodyPart::Threads(model.threads.clone());
    };
    let thread = (model.threads.iter())
        .find(|row| row.active)
        .map(|row| row.id.clone());
    BodyPart::Conversation(Conversation {
        thread,
        messages: transcript.messages.clone(),
        streaming: transcript.streaming.clone(),
        error: transcript.error.clone(),
        empty_hint: transcript.empty_hint.clone(),
    })
}

fn run_of(model: &ChatModel) -> Option<RunBar> {
    model.transcript.as_ref()?.run.clone()
}

impl ChatParts {
    pub(super) fn of(model: &ChatModel) -> Self {
        Self {
            header: Rc::new(header_of(model)),
            body: Rc::new(body_of(model)),
            run: run_of(model).map(Rc::new),
            composer: Rc::new(model.composer.clone()),
        }
    }

    /// Brings every part in step with `model`, keeping each that says what
    /// it said.
    pub(super) fn refresh(&mut self, model: &ChatModel) {
        keep(&mut self.header, header_of(model));
        keep(&mut self.body, body_of(model));
        keep_some(&mut self.run, run_of(model));
        keep(&mut self.composer, model.composer.clone());
    }
}

#[cfg(test)]
mod tests {
    use specular_agent::{Notice, Progress, ProgressKind};
    use specular_testkit::TestApp;

    use super::*;
    use crate::models::same;

    /// The parts of `parts` that are no longer the allocations `before`
    /// has: the views that would be told to draw again.
    fn redrawn(before: &ChatParts, parts: &ChatParts) -> Vec<&'static str> {
        let kept = [
            ("header", Rc::ptr_eq(&before.header, &parts.header)),
            ("body", Rc::ptr_eq(&before.body, &parts.body)),
            ("run", same(before.run.as_ref(), parts.run.as_ref())),
            ("composer", Rc::ptr_eq(&before.composer, &parts.composer)),
        ];
        (kept.iter())
            .filter_map(|(name, kept)| (!kept).then_some(*name))
            .collect()
    }

    #[test]
    fn what_the_agent_says_redraws_the_view_it_is_shown_in_and_no_other() {
        let step = Progress {
            kind: ProgressKind::ToolUse,
            text: "Read src/main.rs".to_owned(),
            label: Some("Reading src/main.rs".to_owned()),
        };
        let mut app = TestApp::with_pages(1);
        app.with_chat_panel().send_chat("look at this");
        let mut parts = ChatParts::of(&app.chat());
        assert!(parts.run.is_some(), "the send started a run");
        let said: [(&str, Notice, &[&str]); 4] = [
            (
                "the first word",
                Notice::TextDelta("One".to_owned()),
                &["body"],
            ),
            (
                "another word",
                Notice::TextDelta(" two".to_owned()),
                &["body"],
            ),
            ("a step", Notice::Progress(step), &["run"]),
            ("a session id", Notice::Session("s1".to_owned()), &[]),
        ];
        for (what, notice, expected) in said {
            let before = parts.clone();
            app.agent_says(notice);
            parts.refresh(&app.chat());
            assert_eq!(redrawn(&before, &parts), expected, "{what}");
            assert_eq!(parts, ChatParts::of(&app.chat()), "{what}");
        }
    }
}
