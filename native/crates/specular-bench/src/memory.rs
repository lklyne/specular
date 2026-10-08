//! Memory of a whole process tree: a shell's main process plus every
//! descendant (CEF or Electron helpers: GPU, network, one renderer per site).
//!
//! The tree comes from `ps`, so the same code measures both shells. Two
//! numbers per sample:
//!
//! - **Footprint** (macOS): `phys_footprint` summed per pid, the memory
//!   verdict's metric. It includes IOSurface and GPU allocations, which is
//!   where a zero-copy compositor's cost lands (see `footprint.rs`).
//! - **RSS**: on every platform, but blind to IOSurface/GPU memory and
//!   double-counting pages shared between processes; kept as a cross-check.

use std::{
    collections::HashMap,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use serde::{Deserialize, Serialize};

use crate::BenchError;
use crate::footprint::phys_footprint;

/// One `ps` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessRow {
    /// Process id.
    pub pid: u32,
    /// Parent process id.
    pub ppid: u32,
    /// Resident set size in KiB.
    pub rss_kb: u64,
}

/// Parses `ps -axo pid=,ppid=,rss=` output, skipping lines that do not hold
/// three integers (a process can exit mid-listing).
pub fn parse_ps(output: &str) -> Vec<ProcessRow> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace().map(str::parse::<u64>);
            let (Some(Ok(pid)), Some(Ok(ppid)), Some(Ok(rss_kb)), None) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                return None;
            };
            Some(ProcessRow {
                pid: u32::try_from(pid).ok()?,
                ppid: u32::try_from(ppid).ok()?,
                rss_kb,
            })
        })
        .collect()
}

/// Resident memory of `root` and all its descendants at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySample {
    /// Root process id.
    pub root_pid: u32,
    /// Processes in the tree, root included.
    pub processes: usize,
    /// Summed RSS in MiB.
    pub rss_mb: f64,
    /// Summed physical footprint in MiB over the processes that could be
    /// read; `None` off macOS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint_mb: Option<f64>,
}

impl MemorySample {
    /// The number memory is judged by: footprint when known, else RSS.
    pub fn verdict_mb(&self) -> f64 {
        self.footprint_mb.unwrap_or(self.rss_mb)
    }
}

/// The rows of `root` and all its descendants, or `None` when `root` is
/// absent.
pub fn tree_rows(rows: &[ProcessRow], root: u32) -> Option<Vec<ProcessRow>> {
    let mut children: HashMap<u32, Vec<&ProcessRow>> = HashMap::new();
    let mut root_row = None;
    for row in rows {
        if row.pid == root {
            root_row = Some(row);
        } else {
            children.entry(row.ppid).or_default().push(row);
        }
    }
    let mut stack = vec![root_row?];
    let mut tree = Vec::new();
    while let Some(row) = stack.pop() {
        tree.push(*row);
        if let Some(kids) = children.get(&row.pid) {
            stack.extend(kids.iter().copied());
        }
    }
    Some(tree)
}

/// Sums RSS (and, via `footprint`, physical footprint) over the tree rooted
/// at `root`, or `None` when `root` is absent.
pub fn tree_sample(
    rows: &[ProcessRow],
    root: u32,
    footprint: impl Fn(u32) -> Option<u64>,
) -> Option<MemorySample> {
    let tree = tree_rows(rows, root)?;
    let rss_kb: u64 = tree.iter().map(|row| row.rss_kb).sum();
    let footprints: Vec<u64> = tree.iter().filter_map(|row| footprint(row.pid)).collect();
    let footprint_mb = (!footprints.is_empty())
        .then(|| footprints.iter().sum::<u64>() as f64 / (1_024.0 * 1_024.0));
    Some(MemorySample {
        root_pid: root,
        processes: tree.len(),
        rss_mb: rss_kb as f64 / 1_024.0,
        footprint_mb,
    })
}

/// Lists processes with `ps` and samples the tree rooted at `root`.
pub fn sample_process_tree(root: u32) -> Result<MemorySample, BenchError> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,rss="])
        .output()
        .map_err(BenchError::ProcessList)?;
    if !output.status.success() {
        return Err(BenchError::ProcessListFailed {
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let rows = parse_ps(&String::from_utf8_lossy(&output.stdout));
    tree_sample(&rows, root, phys_footprint).ok_or(BenchError::NoSuchProcess(root))
}

/// Memory over a run: before the first gesture, after the last, and the
/// peak seen by a background sampler in between.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryReport {
    /// Settled, before any gesture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle: Option<MemorySample>,
    /// Right after the last gesture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end: Option<MemorySample>,
    /// Largest tree sample during the run, by [`MemorySample::verdict_mb`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peak: Option<MemorySample>,
}

/// Samples a process tree on its own thread, so spawning `ps` never lands on
/// the thread whose frame times are being measured.
#[derive(Debug)]
pub struct PeakSampler {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<Option<MemorySample>>,
}

impl PeakSampler {
    /// Starts sampling `root` every `interval`.
    pub fn spawn(root: u32, interval: Duration) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let handle = thread::spawn(move || {
            let mut peak: Option<MemorySample> = None;
            while !flag.load(Ordering::Relaxed) {
                if let Ok(sample) = sample_process_tree(root)
                    && peak.is_none_or(|p| sample.verdict_mb() > p.verdict_mb())
                {
                    peak = Some(sample);
                }
                thread::sleep(interval);
            }
            peak
        });
        Self { stop, handle }
    }

    /// Stops sampling and returns the peak, if any sample succeeded.
    pub fn finish(self) -> Option<MemorySample> {
        self.stop.store(true, Ordering::Relaxed);
        self.handle.join().ok().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PS: &str = "    1     0  1000
  100     1  2048
  101   100  1024
  102   100  1024
  103   101   512
  200     1  9999
garbage line
  104   102
";

    fn no_footprint(_pid: u32) -> Option<u64> {
        None
    }

    #[test]
    fn tree_sample_sums_rss_of_root_and_all_descendants() {
        let sample = tree_sample(&parse_ps(PS), 100, no_footprint).unwrap();
        assert!((sample.rss_mb - (2048.0 + 1024.0 + 1024.0 + 512.0) / 1024.0).abs() < 1e-9);
    }

    #[test]
    fn tree_sample_excludes_siblings_of_root() {
        let sample = tree_sample(&parse_ps(PS), 101, no_footprint).unwrap();
        assert_eq!(sample.processes, 2);
    }

    #[test]
    fn tree_sample_sums_footprint_over_readable_processes() {
        // Process 103 is unreadable; the other three report 1 MiB each.
        let footprint = |pid: u32| (pid != 103).then_some(1_024 * 1_024);
        let sample = tree_sample(&parse_ps(PS), 100, footprint).unwrap();
        assert_eq!(sample.footprint_mb, Some(3.0));
    }

    #[test]
    fn verdict_prefers_footprint_over_rss() {
        let sample = MemorySample {
            root_pid: 1,
            processes: 1,
            rss_mb: 100.0,
            footprint_mb: Some(250.0),
        };
        assert!((sample.verdict_mb() - 250.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sampling_own_process_reports_resident_memory() {
        let sample = sample_process_tree(std::process::id()).unwrap();
        assert!(sample.rss_mb > 0.0);
    }
}
