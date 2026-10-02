//! Frame-interval statistics.

use std::time::Duration;

use serde::{Deserialize, Serialize};

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
pub(crate) fn percentile(sorted: &[f64], p: f64) -> f64 {
    let rank = (p * sorted.len() as f64).ceil() as usize;
    sorted
        .get(rank.saturating_sub(1))
        .copied()
        .unwrap_or_default()
}

/// Frame-timing summary, field-compatible with the ADR 0038 lab's
/// `summarizeFrameIntervals` (`draws`, `drawFps`, `meanFrameMs`,
/// `maxFrameMs`, `longFrames`) plus percentiles the lab did not report.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSummary {
    /// Number of intervals (the lab's `draws`).
    #[serde(rename = "draws", alias = "frames")]
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
    #[expect(
        clippy::float_cmp,
        reason = "nearest-rank percentiles return an input value unchanged"
    )]
    fn percentiles_on_one_to_hundred_are_their_rank() {
        let ms: Vec<u64> = (1..=100).collect();
        let summary = times(&ms).summary(Duration::from_millis(8));
        assert_eq!(
            [
                summary.p50_frame_ms,
                summary.p95_frame_ms,
                summary.p99_frame_ms,
                summary.max_frame_ms
            ],
            [50.0, 95.0, 99.0, 100.0]
        );
    }

    #[test]
    fn percentiles_ignore_recording_order() {
        let summary = times(&[9, 1, 5, 3, 7]).summary(Duration::from_millis(8));
        assert!((summary.p50_frame_ms - 5.0).abs() < 1e-9);
    }

    #[test]
    fn single_interval_is_every_percentile() {
        let summary = times(&[12]).summary(Duration::from_millis(8));
        assert!(
            (summary.p99_frame_ms - 12.0).abs() < 1e-9
                && (summary.p50_frame_ms - 12.0).abs() < 1e-9
        );
    }

    #[test]
    fn mean_matches_arithmetic_mean() {
        let summary = times(&[4, 8, 12]).summary(Duration::from_millis(8));
        assert!((summary.mean_frame_ms - 8.0).abs() < 1e-9);
    }

    #[test]
    fn interval_exactly_at_threshold_is_not_long() {
        // 1.5 x 8 ms = 12 ms; the lab counts strictly greater.
        let summary = times(&[12]).summary(Duration::from_millis(8));
        assert_eq!(summary.long_frames, 0);
    }

    #[test]
    fn summary_serializes_frame_count_as_lab_draws() {
        let json = serde_json::to_value(times(&[8]).summary(Duration::from_millis(8))).unwrap();
        assert_eq!(json["draws"], 1);
    }

    #[test]
    fn summary_deserializes_legacy_frames_field() {
        let summary: FrameSummary = serde_json::from_str(
            r#"{"frames":3,"drawFps":1,"meanFrameMs":1,"p50FrameMs":1,"p95FrameMs":1,"p99FrameMs":1,"maxFrameMs":1,"longFrames":0}"#,
        )
        .unwrap();
        assert_eq!(summary.frames, 3);
    }

    #[test]
    fn draw_fps_is_frames_over_total_time() {
        let summary = times(&[10, 10, 10, 10]).summary(Duration::from_millis(8));
        assert!((summary.draw_fps - 100.0).abs() < 1e-9);
    }
}
