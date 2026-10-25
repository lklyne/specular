//! A process tree's memory, one row a process: which of a shell's helpers
//! (the GPU process, each renderer, the network service) holds what.

use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::BenchError;
use crate::footprint::phys_footprint;
use crate::memory::{ProcessRow, parse_ps, tree_rows};

/// One process of the tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessMemory {
    /// Process id.
    pub(crate) pid: u32,
    /// What the process is, from its command line: `browser` for the root,
    /// else Chromium's `--type` (`renderer`, `gpu-process`), with the
    /// service's name for a `utility`.
    pub(crate) kind: String,
    /// Resident set size in MiB.
    pub(crate) rss_mb: f64,
    /// Physical footprint in MiB; `None` off macOS or when it cannot be
    /// read.
    pub(crate) footprint_mb: Option<f64>,
}

/// What a process is, from its command line. `root` is the shell itself.
pub(crate) fn process_kind(command: &str, root: bool) -> String {
    let value_of = |flag: &str| {
        command
            .split_whitespace()
            .find_map(|argument| argument.strip_prefix(flag))
    };
    match (value_of("--type="), root) {
        (Some("utility"), _) => {
            let service = value_of("--utility-sub-type=").unwrap_or("unknown");
            // `network.mojom.NetworkService` reads as `network`.
            let name = service.split('.').next().unwrap_or(service);
            format!("utility:{name}")
        }
        (Some(kind), _) => kind.to_owned(),
        (None, true) => "browser".to_owned(),
        (None, false) => "other".to_owned(),
    }
}

/// The rows of `tree` with their kinds, largest footprint first. `command`
/// gives a process's command line and `footprint` its physical footprint in
/// bytes.
pub(crate) fn breakdown(
    tree: &[ProcessRow],
    root: u32,
    command: impl Fn(u32) -> String,
    footprint: impl Fn(u32) -> Option<u64>,
) -> Vec<ProcessMemory> {
    let mut rows: Vec<ProcessMemory> = (tree.iter())
        .map(|row| ProcessMemory {
            pid: row.pid,
            kind: process_kind(&command(row.pid), row.pid == root),
            rss_mb: row.rss_kb as f64 / 1_024.0,
            footprint_mb: footprint(row.pid).map(|bytes| bytes as f64 / (1_024.0 * 1_024.0)),
        })
        .collect();
    let weight = |row: &ProcessMemory| row.footprint_mb.unwrap_or(row.rss_mb);
    rows.sort_by(|a, b| weight(b).total_cmp(&weight(a)));
    rows
}

/// Lists the tree rooted at `root` with `ps` and breaks its memory down.
pub fn sample_breakdown(root: u32) -> Result<Vec<ProcessMemory>, BenchError> {
    let ps = |args: &[&str]| {
        let output = Command::new("ps")
            .args(args)
            .output()
            .map_err(BenchError::ProcessList)?;
        Ok::<_, BenchError>(String::from_utf8_lossy(&output.stdout).into_owned())
    };
    let rows = parse_ps(&ps(&["-axo", "pid=,ppid=,rss="])?);
    let tree = tree_rows(&rows, root).ok_or(BenchError::NoSuchProcess(root))?;
    let command = |pid: u32| ps(&["-o", "command=", "-p", &pid.to_string()]).unwrap_or_default();
    Ok(breakdown(&tree, root, command, phys_footprint))
}
