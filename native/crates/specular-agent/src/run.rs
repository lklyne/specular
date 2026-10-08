use crate::stream::Progress;
use crate::thread::Image;

/// The run log keeps this many lines; older ones drop.
pub const MAX_EVENTS: usize = 200;

/// Whether a run is going or stopped on an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunState {
    /// The agent is working.
    Running,
    /// It failed; the queue stays queued and the next send replaces this run.
    Failed(String),
}

/// A run in flight (or just failed) for one thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// What the agent did, newest last.
    pub events: Vec<Progress>,
    /// The assistant text streaming in.
    pub text: String,
    /// Going or failed.
    pub state: RunState,
    pub(crate) session: Option<String>,
    pub(crate) fallback_prompt: String,
    pub(crate) resumed: bool,
    pub(crate) images: Vec<Image>,
}

impl Run {
    pub(crate) fn new(fallback_prompt: String, resumed: bool, images: Vec<Image>) -> Self {
        Self {
            events: Vec::new(),
            text: String::new(),
            state: RunState::Running,
            session: None,
            fallback_prompt,
            resumed,
            images,
        }
    }

    pub(crate) fn push(&mut self, event: Progress) {
        self.events.push(event);
        if self.events.len() > MAX_EVENTS {
            self.events.remove(0);
        }
    }

    /// What the run bar says: the latest labelled step, or `Starting`.
    pub fn current_label(&self) -> &str {
        self.events
            .iter()
            .rev()
            .find_map(|e| e.label.as_deref())
            .unwrap_or("Starting")
    }
}
