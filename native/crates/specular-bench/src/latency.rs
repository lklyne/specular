//! Input latency: an event's timestamp to the first presented frame that
//! reflects it.
//!
//! Each event gets a sequence number. Whatever carries the event's effect —
//! the camera generation for a wheel gesture, a page's next frame for
//! forwarded input — carries that number forward, and the frame that draws it
//! resolves every pending event up to it. Coalesced events (several wheel
//! ticks folded into one frame) each get their own sample, measured from
//! their own timestamp, so coalescing shows up as latency rather than hiding.

use std::{collections::VecDeque, time::Duration, time::Instant};

use serde::{Deserialize, Serialize};

use crate::stats::percentile;

/// At most this many events wait for a frame; past it the oldest is dropped
/// and counted as unresolved, so events whose effect never presents (a key
/// the page ignored) cannot grow the queue without bound.
const MAX_PENDING: usize = 1_024;

/// Sequence number of one recorded input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputSeq(pub u64);

/// Collects event-to-present samples.
#[derive(Debug, Clone, Default)]
pub struct LatencyTracker {
    next: u64,
    pending: VecDeque<(InputSeq, Instant)>,
    samples: Vec<Duration>,
    dropped: usize,
}

impl LatencyTracker {
    /// An empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an input event that happened at `at` (the OS event timestamp
    /// when available, else when the shell received it) and returns the
    /// number its effect must carry.
    pub fn event(&mut self, at: Instant) -> InputSeq {
        let seq = InputSeq(self.next);
        self.next += 1;
        if self.pending.len() == MAX_PENDING {
            self.pending.pop_front();
            self.dropped += 1;
        }
        self.pending.push_back((seq, at));
        seq
    }

    /// A frame presented at `at` reflects every event up to and including
    /// `reflected`. Returns how many pending events it resolved.
    pub fn presented(&mut self, reflected: InputSeq, at: Instant) -> usize {
        let mut resolved = 0;
        while let Some(&(seq, sent)) = self.pending.front()
            && seq <= reflected
        {
            self.pending.pop_front();
            self.samples.push(at.saturating_duration_since(sent));
            resolved += 1;
        }
        resolved
    }

    /// Events not yet reflected by a presented frame.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    /// Resolved latencies, in resolution order.
    pub fn samples(&self) -> &[Duration] {
        &self.samples
    }

    /// Summary of the resolved samples.
    pub fn summary(&self) -> LatencySummary {
        LatencySummary::from_samples(&self.samples, self.dropped + self.pending.len())
    }
}

/// Distribution of input-to-present latencies.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencySummary {
    /// Resolved samples.
    pub samples: usize,
    /// Events never reflected by a presented frame.
    #[serde(default)]
    pub unresolved: usize,
    /// Mean latency.
    pub mean_ms: f64,
    /// Median latency.
    pub p50_ms: f64,
    /// 95th percentile latency.
    pub p95_ms: f64,
    /// Worst latency.
    pub max_ms: f64,
}

impl LatencySummary {
    /// Summarises `samples`, noting `unresolved` events alongside.
    pub fn from_samples(samples: &[Duration], unresolved: usize) -> Self {
        let mut ms: Vec<f64> = samples.iter().map(|d| d.as_secs_f64() * 1_000.0).collect();
        if ms.is_empty() {
            return Self {
                unresolved,
                ..Self::default()
            };
        }
        ms.sort_by(f64::total_cmp);
        Self {
            samples: ms.len(),
            unresolved,
            mean_ms: ms.iter().sum::<f64>() / ms.len() as f64,
            p50_ms: percentile(&ms, 0.50),
            p95_ms: percentile(&ms, 0.95),
            max_ms: ms.last().copied().unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn latency_runs_from_event_to_presenting_frame() {
        let start = Instant::now();
        let mut tracker = LatencyTracker::new();
        let seq = tracker.event(start);
        tracker.presented(seq, start + MS * 12);
        assert_eq!(tracker.samples(), [MS * 12]);
    }

    #[test]
    fn coalesced_events_each_measure_from_their_own_timestamp() {
        let start = Instant::now();
        let mut tracker = LatencyTracker::new();
        tracker.event(start);
        let last = tracker.event(start + MS * 4);
        tracker.presented(last, start + MS * 10);
        assert_eq!(tracker.samples(), [MS * 10, MS * 6]);
    }

    #[test]
    fn queue_overflow_counts_dropped_events_as_unresolved() {
        let start = Instant::now();
        let mut tracker = LatencyTracker::new();
        for _ in 0..MAX_PENDING + 3 {
            tracker.event(start);
        }
        assert_eq!(tracker.pending(), MAX_PENDING);
        assert_eq!(tracker.summary().unresolved, MAX_PENDING + 3);
    }

    #[test]
    fn summary_percentiles_use_nearest_rank() {
        let samples: Vec<Duration> = (1..=20).map(|n| MS * n).collect();
        let summary = LatencySummary::from_samples(&samples, 0);
        assert_eq!(
            [summary.p50_ms, summary.p95_ms, summary.max_ms],
            [10.0, 19.0, 20.0]
        );
    }
}
