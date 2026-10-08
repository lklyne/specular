use std::path::Path;
use std::sync::Arc;

use specular_agent::RunRequest;

use super::{ClaudeCli, Scripted};

/// Called from a worker thread whenever a run has output, so a loop that
/// sleeps between events can wake and poll.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// Why a run could not start.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    /// No `claude` program was found.
    #[error(
        "Claude Code was not found. Install it from https://claude.com/code, run `claude` in a terminal once to sign in, then retry."
    )]
    NotInstalled,
    /// The program exists but would not start.
    #[error(
        "Claude Code could not start ({0}). Install it from https://claude.com/code, run `claude` in a terminal once to sign in, then retry."
    )]
    Spawn(#[source] std::io::Error),
    /// The agent is switched off in this run.
    #[error("the agent is off in this run")]
    Off,
    /// A script file could not be read or understood.
    #[error("agent script: {0}")]
    Script(String),
}

/// One thing a run did that is not the stream's meaning yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// A line of stdout, without its newline.
    Stdout(String),
    /// A line of stderr, without its newline.
    Stderr(String),
    /// The process ended. Arrives once, after every line.
    Exit {
        /// Whether it ended with code 0.
        success: bool,
        /// How, in words.
        detail: String,
    },
}

/// Starts agent runs. The one seam between the app and the `claude` process.
pub trait AgentBackend: Send {
    /// Starts a run in `cwd`.
    fn start(
        &mut self,
        request: &RunRequest,
        cwd: &Path,
    ) -> Result<Box<dyn AgentProcess>, AgentError>;

    /// Gives the backend the hook to call when output arrives.
    fn set_waker(&mut self, _waker: Waker) {}
}

/// One run in flight.
pub trait AgentProcess: Send {
    /// The output that arrived since the last call, never blocking. `Exit`
    /// comes once, when the process ends.
    fn poll(&mut self) -> Vec<Output>;

    /// Stops it. Idempotent.
    fn kill(&mut self);
}

/// What headless runs and tests get: nothing can start a real run.
#[derive(Debug, Default, Clone, Copy)]
pub struct Disabled;

impl AgentBackend for Disabled {
    fn start(&mut self, _: &RunRequest, _: &Path) -> Result<Box<dyn AgentProcess>, AgentError> {
        Err(AgentError::Off)
    }
}

/// `SPECULAR_AGENT_SCRIPT=<path>` plays that script in real time, with no
/// CLI. Otherwise the real `claude`.
pub fn backend_from_env() -> Box<dyn AgentBackend> {
    if let Some(path) = std::env::var_os("SPECULAR_AGENT_SCRIPT") {
        match Scripted::from_file(Path::new(&path)) {
            Ok(script) => return Box::new(script),
            Err(error) => tracing::warn!("SPECULAR_AGENT_SCRIPT ignored: {error}"),
        }
    }
    Box::new(ClaudeCli::new())
}
