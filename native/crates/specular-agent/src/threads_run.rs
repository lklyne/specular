use crate::cli::RunRequest;
use crate::prompt::{PromptContext, follow_up_prompt, thread_prompt};
use crate::reply::parse_output;
use crate::run::{Run, RunState};
use crate::stream::{Notice, Progress, ProgressKind};
use crate::thread::{Message, Role, Status, ThreadId};
use crate::threads::{Changed, Threads, queued_of};

/// A run the caller should start, and the thread file it changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    /// The `claude` run to spawn.
    pub request: RunRequest,
    /// What to write.
    pub changed: Changed,
}

/// What a [`Notice`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// There was no run to apply it to.
    Ignored,
    /// The run's log or text changed; nothing needs writing.
    Progressed,
    /// The agent answered. When `has_queued_turn`, call `begin_run` again to
    /// send what was typed during the run.
    Finished {
        /// Follow-ups wait in the queue.
        has_queued_turn: bool,
        /// What to write.
        changed: Changed,
    },
    /// The resume failed: start this request instead (full prompt, no resume).
    Retry {
        /// The run to spawn.
        request: RunRequest,
        /// What to write.
        changed: Changed,
    },
    /// The run stopped on an error and shows it; the queue stays queued.
    Failed,
    /// The run is gone and nothing was appended.
    Cancelled,
}

impl Threads {
    /// The comment behind the newest queued message that has one: where a
    /// drained turn is aimed.
    pub fn last_queued_annotation(&self, id: &ThreadId) -> Option<&str> {
        self.queued(id)
            .into_iter()
            .rev()
            .find_map(|m| m.annotation_id.as_deref())
    }

    /// Sends the queue. `None` while a run is in flight (the turn stays
    /// queued) or when no user message has content.
    pub fn begin_run(&mut self, id: &ThreadId, now: &str, ctx: &PromptContext) -> Option<Started> {
        if self.is_running(id) {
            return None;
        }
        let thread = self.items.iter_mut().find(|t| &t.id == id)?;
        if !thread
            .messages
            .iter()
            .any(|m| m.role == Role::User && m.has_content())
        {
            return None;
        }
        let queued = queued_of(thread);
        let has_turn = !queued.is_empty();
        let turn = queued
            .iter()
            .map(|m| m.text.trim())
            .filter(|t| !t.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        let images: Vec<_> = (queued.iter())
            .flat_map(|m| m.images.iter().cloned())
            .collect();
        let image_count = images.len();
        let mut annotations: Vec<String> = Vec::new();
        for a in queued.iter().filter_map(|m| m.annotation_id.as_ref()) {
            if !annotations.contains(a) {
                annotations.push(a.clone());
            }
        }
        let resume = if thread.status == Status::Open {
            thread.claude_session_id.clone()
        } else {
            None
        };

        for message in &mut thread.messages {
            message.queued = false;
        }
        now.clone_into(&mut thread.updated_at);
        let full = thread_prompt(thread, ctx);
        let prompt = if resume.is_some() && has_turn {
            follow_up_prompt(&turn, ctx, image_count, &annotations)
        } else {
            full.clone()
        };
        let cwd = ctx.write_target.cwd().map(str::to_owned);
        self.runs.insert(
            id.clone(),
            Run::new(full, resume.is_some(), images.clone(), cwd.clone()),
        );
        Some(Started {
            request: RunRequest {
                thread: id.clone(),
                prompt,
                resume,
                images,
                cwd,
            },
            changed: Changed::thread(id),
        })
    }

    /// Applies one thing the stream said. `message_id` names the agent
    /// message a `Finished` appends.
    pub fn on_notice(
        &mut self,
        id: &ThreadId,
        notice: Notice,
        message_id: &str,
        now: &str,
    ) -> Outcome {
        if notice == Notice::Cancelled {
            return if self.runs.remove(id).is_some() {
                Outcome::Cancelled
            } else {
                Outcome::Ignored
            };
        }
        let Some(run) = self
            .runs
            .get_mut(id)
            .filter(|r| r.state == RunState::Running)
        else {
            return Outcome::Ignored;
        };
        match notice {
            Notice::Session(session) => run.session = Some(session),
            Notice::Progress(event) => run.push(event),
            Notice::Text(text) => run.text = text,
            Notice::TextDelta(delta) => run.text.push_str(&delta),
            Notice::Finished { text } => return self.finish(id, &text, message_id, now),
            Notice::Failed { error } => return self.fail(id, error),
            Notice::Cancelled => {}
        }
        Outcome::Progressed
    }

    fn finish(&mut self, id: &ThreadId, text: &str, message_id: &str, now: &str) -> Outcome {
        let session = self.runs.remove(id).and_then(|r| r.session);
        let Some(thread) = self.items.iter_mut().find(|t| &t.id == id) else {
            return Outcome::Ignored;
        };
        thread.messages.push(Message {
            id: message_id.to_owned(),
            role: Role::Agent,
            text: parse_output(text).summary,
            created_at: now.to_owned(),
            queued: false,
            annotation_id: None,
            images: Vec::new(),
        });
        thread.status = Status::Open;
        now.clone_into(&mut thread.updated_at);
        if session.is_some() {
            thread.claude_session_id = session;
        }
        Outcome::Finished {
            has_queued_turn: !queued_of(thread).is_empty(),
            changed: Changed::thread(id),
        }
    }

    fn fail(&mut self, id: &ThreadId, error: String) -> Outcome {
        let Some(run) = self.runs.get_mut(id) else {
            return Outcome::Ignored;
        };
        if !run.resumed {
            run.state = RunState::Failed(error);
            return Outcome::Failed;
        }
        run.resumed = false;
        run.session = None;
        run.text.clear();
        run.push(Progress {
            kind: ProgressKind::System,
            text: "Could not resume prior session — starting fresh.".into(),
            label: None,
        });
        let request = RunRequest {
            thread: id.clone(),
            prompt: run.fallback_prompt.clone(),
            resume: None,
            images: run.images.clone(),
            cwd: run.cwd.clone(),
        };
        if let Some(thread) = self.items.iter_mut().find(|t| &t.id == id) {
            thread.claude_session_id = None;
        }
        Outcome::Retry {
            request,
            changed: Changed::thread(id),
        }
    }
}
