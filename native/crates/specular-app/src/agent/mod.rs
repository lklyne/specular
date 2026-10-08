//! The canvas agent's I/O: the `claude` process, the files of a thread and
//! the runs in flight.
//!
//! [`AgentBackend`] is the one seam to the process. [`AgentRuns`] owns one
//! run per thread, applies every rule about how a run ends, and hands the
//! runtime [`Notice`](specular_agent::Notice)s from a non-blocking
//! [`poll`](AgentRuns::poll), once per loop turn while it is not
//! [idle](AgentRuns::is_idle). A [`Waker`] given to the backend is called
//! whenever output arrives, so a sleeping loop can wake to poll.

mod backend;
mod claude;
mod files;
mod runs;
mod scripted;

pub use self::backend::{
    AgentBackend, AgentError, AgentProcess, Disabled, Output, Waker, backend_from_env,
};
pub use self::claude::ClaudeCli;
pub use self::files::{load, write_index, write_thread};
pub use self::runs::{AgentRuns, DEFAULT_TIMEOUT};
pub use self::scripted::{Scripted, StartedRuns};
