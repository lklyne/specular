//! Errors raised by the bench library.

use std::{io, path::PathBuf};

/// Everything the bench library can fail on.
#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    /// A profile id that is neither an Electron id nor a lab alias.
    #[error("unknown gesture profile `{0}`")]
    UnknownProfile(String),
    /// A file could not be read.
    #[error("cannot read {path}")]
    Read {
        /// The file.
        path: PathBuf,
        /// Why.
        #[source]
        source: io::Error,
    },
    /// Input that is not the JSON shape expected.
    #[error("{what} is not valid JSON")]
    Json {
        /// What was being parsed, for the message.
        what: String,
        /// Why.
        #[source]
        source: serde_json::Error,
    },
    /// A results file held no gesture phases.
    #[error("{0} contains no gesture phases")]
    NoPhases(String),
    /// A Chromium trace held no presented frames to time.
    #[error("trace has no `{event}` events on the selected thread")]
    NoPresents {
        /// The event name searched for.
        event: &'static str,
    },
    /// Fewer gesture bursts in a trace than profiles to assign them to.
    #[error(
        "found {found} gesture bursts for {expected} profiles; pages may be animating \
         through the phase gaps (run one profile per trace) or `--gap-ms` is too large"
    )]
    Segmentation {
        /// Bursts found.
        found: usize,
        /// Profiles requested.
        expected: usize,
    },
    /// `ps` could not be run or exited unsuccessfully.
    #[error("process listing failed: {0}")]
    ProcessList(String),
    /// The process whose tree was to be sampled is not running.
    #[error("process {0} is not running")]
    NoSuchProcess(u32),
}
