//! Errors raised by the bench library.

use std::io;

/// Everything the bench library can fail on.
#[derive(Debug, thiserror::Error)]
pub enum BenchError {
    /// A profile id that names no gesture profile.
    #[error("unknown gesture profile `{0}`")]
    UnknownProfile(String),
    /// A paint policy name other than `electron-lod` or `full-rate`.
    #[error("unknown paint policy `{0}` (expected electron-lod or full-rate)")]
    UnknownPaintPolicy(String),
    /// `ps` could not be run.
    #[error("cannot run `ps`")]
    ProcessList(#[source] io::Error),
    /// `ps` ran but exited unsuccessfully.
    #[error("`ps` failed: {stderr}")]
    ProcessListFailed {
        /// What `ps` printed on stderr.
        stderr: String,
    },
    /// The process whose tree was to be sampled is not running.
    #[error("process {0} is not running")]
    NoSuchProcess(u32),
}
