//! Resident memory of a whole process tree: a shell's main process plus every
//! descendant (CEF or Electron helpers: GPU, network, one renderer per site).
//!
//! Sampled with `ps` rather than a platform API so the same code measures
//! both shells, on macOS and Linux, with no extra dependency. `ps` reports
//! RSS, which on macOS excludes compressed and swapped pages; `footprint`
//! is closer to what Activity Monitor shows but is not scriptable per tree,
//! so treat absolute numbers as RSS and compare shells only with each other.

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
}

/// Sums RSS over the tree rooted at `root`, or `None` when `root` is absent.
pub fn tree_sample(rows: &[ProcessRow], root: u32) -> Option<MemorySample> {
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
    let mut processes = 0;
    let mut rss_kb = 0;
    while let Some(row) = stack.pop() {
        processes += 1;
        rss_kb += row.rss_kb;
        if let Some(kids) = children.get(&row.pid) {
            stack.extend(kids.iter().copied());
        }
    }
    Some(MemorySample {
        root_pid: root,
        processes,
        rss_mb: rss_kb as f64 / 1_024.0,
    })
}

/// Lists processes with `ps` and samples the tree rooted at `root`.
pub fn sample_process_tree(root: u32) -> Result<MemorySample, BenchError> {
    let output = Command::new("ps")
        .args(["-axo", "pid=,ppid=,rss="])
        .output()
        .map_err(|error| BenchError::ProcessList(error.to_string()))?;
    if !output.status.success() {
        return Err(BenchError::ProcessList(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    let rows = parse_ps(&String::from_utf8_lossy(&output.stdout));
    tree_sample(&rows, root).ok_or(BenchError::NoSuchProcess(root))
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
    /// Largest tree RSS sampled during the run.
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
                    && peak.is_none_or(|p| sample.rss_mb > p.rss_mb)
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

    #[test]
    fn parse_ps_skips_malformed_lines() {
        assert_eq!(parse_ps(PS).len(), 6);
    }

    #[test]
    fn tree_sample_sums_root_and_all_descendants() {
        let sample = tree_sample(&parse_ps(PS), 100).unwrap();
        assert_eq!(
            (sample.processes, sample.rss_mb),
            (4, (2048.0 + 1024.0 + 1024.0 + 512.0) / 1024.0)
        );
    }

    #[test]
    fn tree_sample_excludes_siblings_of_root() {
        let sample = tree_sample(&parse_ps(PS), 101).unwrap();
        assert_eq!(sample.processes, 2);
    }

    #[test]
    fn tree_sample_of_missing_root_is_none() {
        assert_eq!(tree_sample(&parse_ps(PS), 4242), None);
    }

    #[test]
    fn sampling_own_process_counts_at_least_itself() {
        let sample = sample_process_tree(std::process::id()).unwrap();
        assert!(sample.processes >= 1 && sample.rss_mb > 0.0);
    }

    #[test]
    fn peak_sampler_reports_own_process() {
        let sampler = PeakSampler::spawn(std::process::id(), Duration::from_millis(5));
        thread::sleep(Duration::from_millis(30));
        assert!(sampler.finish().is_some());
    }
}
