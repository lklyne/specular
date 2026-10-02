//! Frame-interval statistics.

use std::time::Duration;

use serde::Serialize;

/// A frame counts as long when its interval exceeds this multiple of the
/// display budget — i.e. at least one refresh was visibly missed.
pub const LONG_FRAME_FACTOR: f64 = 1.5;

/// Collected presented-frame intervals for one profile run.
#[derive(Debug, Clone, Default)]
pub struct FrameTimes {
    intervals: Vec<Duration>,
}

impl FrameTimes {
    /// An empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the time between two consecutive presented frames.
    pub fn record(&mut self, interval: Duration) {
        self.intervals.push(interval);
    }

    /// Recorded intervals, in order.
    pub fn intervals(&self) -> &[Duration] {
        &self.intervals
    }

    /// Summarises against a display frame `budget` (e.g. 8.33 ms at 120 Hz).
    pub fn summary(&self, budget: Duration) -> FrameSummary {
        let mut ms: Vec<f64> = self
            .intervals
            .iter()
            .map(|d| d.as_secs_f64() * 1_000.0)
            .collect();
        if ms.is_empty() {
            return FrameSummary::default();
        }
        ms.sort_by(f64::total_cmp);
        let total: f64 = ms.iter().sum();
        let mean = total / ms.len() as f64;
        let long_threshold = budget.as_secs_f64() * 1_000.0 * LONG_FRAME_FACTOR;
        FrameSummary {
            frames: ms.len(),
            draw_fps: if total > 0.0 {
                ms.len() as f64 * 1_000.0 / total
            } else {
                0.0
            },
            mean_frame_ms: mean,
            p50_frame_ms: percentile(&ms, 0.50),
            p95_frame_ms: percentile(&ms, 0.95),
            p99_frame_ms: percentile(&ms, 0.99),
            max_frame_ms: ms.last().copied().unwrap_or_default(),
            long_frames: ms.iter().filter(|&&v| v > long_threshold).count(),
        }
    }
}

/// Nearest-rank percentile over sorted values (`sorted[ceil(p*n) - 1]`, as
/// the Electron `computeBuildStats` does).
fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = (p * sorted.len() as f64).ceil() as usize;
    sorted
        .get(rank.saturating_sub(1))
        .copied()
        .unwrap_or_default()
}

/// Frame-timing summary, field-compatible with the ADR 0038 lab tables.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSummary {
    /// Number of intervals.
    pub frames: usize,
    /// Presented frames per second over the run.
    pub draw_fps: f64,
    /// Mean interval.
    pub mean_frame_ms: f64,
    /// Median interval.
    pub p50_frame_ms: f64,
    /// 95th percentile interval.
    pub p95_frame_ms: f64,
    /// 99th percentile interval.
    pub p99_frame_ms: f64,
    /// Worst interval.
    pub max_frame_ms: f64,
    /// Intervals over [`LONG_FRAME_FACTOR`] x budget.
    pub long_frames: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn times(ms: &[u64]) -> FrameTimes {
        let mut t = FrameTimes::new();
        for &m in ms {
            t.record(Duration::from_millis(m));
        }
        t
    }

    #[test]
    fn empty_recorder_summarises_to_zero() {
        assert_eq!(
            FrameTimes::new().summary(Duration::from_millis(8)),
            FrameSummary::default()
        );
    }

    #[test]
    fn long_frames_counts_intervals_over_one_and_half_budgets() {
        let summary = times(&[8, 8, 13, 20]).summary(Duration::from_millis(8));
        assert_eq!(summary.long_frames, 2);
    }

    #[test]
    fn p95_uses_nearest_rank() {
        let summary = times(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).summary(Duration::from_millis(8));
        assert!((summary.p95_frame_ms - 10.0).abs() < 1e-9);
    }

    #[test]
    fn draw_fps_is_frames_over_total_time() {
        let summary = times(&[10, 10, 10, 10]).summary(Duration::from_millis(8));
        assert!((summary.draw_fps - 100.0).abs() < 1e-9);
    }
}
