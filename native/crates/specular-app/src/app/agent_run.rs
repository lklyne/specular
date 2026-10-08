//! The agent effects: the threads on disk and the `claude` runs.

use specular_interact::{RunRequest, ThreadId};

use super::runtime::{Runtime, ShellWindow};

#[expect(
    clippy::unused_self,
    reason = "placeholders: the next step reads and writes through the runtime"
)]
impl<W: ShellWindow> Runtime<W> {
    /// Reads `.specular/threads/` and answers with `Event::ThreadsLoaded`.
    pub(super) fn load_threads(&mut self) {
        // wired in the next step
        tracing::debug!("load_threads: not wired yet");
    }

    /// Writes one thread's file.
    pub(super) fn write_thread(&mut self, thread: &ThreadId) {
        // wired in the next step
        tracing::debug!(thread = thread.as_str(), "write_thread: not wired yet");
    }

    /// Writes `.specular/threads/index.json`.
    pub(super) fn write_thread_index(&mut self) {
        // wired in the next step
        tracing::debug!("write_thread_index: not wired yet");
    }

    /// Starts the `claude` CLI for a thread; its output comes back as
    /// `Event::Agent`.
    pub(super) fn run_agent(&mut self, request: &RunRequest) {
        // wired in the next step
        tracing::debug!(thread = request.thread.as_str(), "run_agent: not wired yet");
    }

    /// Stops a thread's run and answers with `Notice::Cancelled`.
    pub(super) fn cancel_agent(&mut self, thread: &ThreadId) {
        // wired in the next step
        tracing::debug!(thread = thread.as_str(), "cancel_agent: not wired yet");
    }
}
