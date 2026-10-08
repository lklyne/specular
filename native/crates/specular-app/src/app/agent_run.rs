//! The agent effects: the threads on disk and the `claude` runs.

use specular_interact::{Event, RunRequest, ThreadId, iso8601};

use super::runtime::{Runtime, ShellWindow, unix_ms};
use crate::agent::{self, AgentBackend, AgentRuns, Disabled};

/// The runs of a session. A test and a benchmark get none, so neither can
/// start `claude`.
pub(super) fn start_runs(real: bool) -> AgentRuns {
    let backend: Box<dyn AgentBackend> = if real && !cfg!(test) {
        agent::backend_from_env()
    } else {
        Box::new(Disabled)
    };
    AgentRuns::new(backend)
}

impl<W: ShellWindow> Runtime<W> {
    /// Reads `.specular/threads/` and answers with `Event::ThreadsLoaded`.
    pub(super) fn load_threads(&mut self) {
        let Some(space) = self.space.as_deref() else {
            return;
        };
        let (threads, index) = agent::load(space, &iso8601(unix_ms()));
        self.dispatch(Event::ThreadsLoaded { threads, index });
    }

    /// Writes one thread's file.
    pub(super) fn write_thread(&self, thread: &ThreadId) {
        let (Some(space), Some(thread)) = (self.space.as_deref(), self.app.threads().get(thread))
        else {
            return;
        };
        if let Err(error) = agent::write_thread(space, thread) {
            tracing::warn!(thread = thread.id.as_str(), "thread not written: {error}");
        }
    }

    /// Writes `.specular/threads/index.json`.
    pub(super) fn write_thread_index(&self) {
        let Some(space) = self.space.as_deref() else {
            return;
        };
        if let Err(error) = agent::write_index(space, &self.app.thread_index_json()) {
            tracing::warn!("thread index not written: {error}");
        }
    }

    /// Starts the `claude` CLI for a thread in the space folder; its output
    /// comes back as `Event::Agent`.
    pub(super) fn run_agent(&mut self, request: &RunRequest) {
        self.agents.start(request, self.space.as_deref());
    }

    /// Stops a thread's run, which answers with `Notice::Cancelled`.
    pub(super) fn cancel_agent(&mut self, thread: &ThreadId) {
        self.agents.cancel(thread);
    }

    /// Hands on what the runs reported since the last turn.
    pub(super) fn take_agent_notices(&mut self) {
        for (thread, notice) in self.agents.poll() {
            self.demand.changed();
            self.dispatch(Event::Agent { thread, notice });
        }
    }

    /// Whether a run is in flight, so the loop must keep turning to hear it.
    pub(super) fn agent_running(&self) -> bool {
        !self.agents.is_idle()
    }
}
