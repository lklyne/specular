//! Frame pacing, measured as ADR 0040 measured it: a timestamp after each
//! present, and the intervals between them. These are CPU-side times, not
//! presented times from the display.

use std::time::{Duration, Instant};

/// Timestamps of presented frames, summarised every so often.
#[derive(Debug)]
pub(crate) struct Pacing {
    stamps: Vec<Instant>,
    every: Duration,
    hz: f64,
}

impl Pacing {
    /// Logs a summary every `every`, counting a frame late against a
    /// display refreshing at `hz`.
    pub(crate) fn new(every: Duration, hz: f64) -> Self {
        Self {
            stamps: Vec::new(),
            every,
            hz,
        }
    }

    /// From `SPECULAR_FRAME_LOG=<seconds>`; absent or unreadable is off.
    pub(crate) fn from_env() -> Option<Self> {
        let seconds: f64 = std::env::var("SPECULAR_FRAME_LOG").ok()?.parse().ok()?;
        let hz = std::env::var("SPECULAR_FRAME_HZ")
            .ok()
            .and_then(|hz| hz.parse().ok())
            .unwrap_or(120.0);
        (seconds > 0.0).then(|| Self::new(Duration::from_secs_f64(seconds), hz))
    }

    /// Notes a frame presented now, and logs the window when it is full.
    pub(crate) fn mark(&mut self) {
        let now = Instant::now();
        self.stamps.push(now);
        let full =
            (self.stamps.first()).is_some_and(|first| now.duration_since(*first) >= self.every);
        if full {
            if let Some(summary) = summarise(&self.stamps, self.hz) {
                tracing::info!(
                    frames = summary.frames,
                    fps = summary.fps,
                    p50_ms = summary.p50_ms,
                    p99_ms = summary.p99_ms,
                    max_ms = summary.max_ms,
                    late = summary.late,
                    very_late = summary.very_late,
                    "canvas frame pacing"
                );
            }
            self.stamps.clear();
        }
    }
}

/// Interval statistics for one window of frames.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Summary {
    frames: usize,
    fps: f64,
    p50_ms: f64,
    p99_ms: f64,
    max_ms: f64,
    /// Intervals over 1.5 frames.
    late: usize,
    /// Intervals over 2.5 frames.
    very_late: usize,
}

fn summarise(stamps: &[Instant], hz: f64) -> Option<Summary> {
    let mut gaps: Vec<f64> = stamps
        .windows(2)
        .map(|pair| pair[1].duration_since(pair[0]).as_secs_f64() * 1000.0)
        .collect();
    if gaps.len() < 2 {
        return None;
    }
    let total: f64 = gaps.iter().sum();
    let frame = 1000.0 / hz;
    let late = gaps.iter().filter(|&&gap| gap > frame * 1.5).count();
    let very_late = gaps.iter().filter(|&&gap| gap > frame * 2.5).count();
    gaps.sort_by(f64::total_cmp);
    let at = |q: f64| gaps[((gaps.len() - 1) as f64 * q).round() as usize];
    let round = |value: f64| (value * 100.0).round() / 100.0;
    Some(Summary {
        frames: gaps.len(),
        fps: round(gaps.len() as f64 / (total / 1000.0)),
        p50_ms: round(at(0.5)),
        p99_ms: round(at(0.99)),
        max_ms: round(at(1.0)),
        late,
        very_late,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dropped_frame_is_late_and_two_are_very_late() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        // 10 ms frames at 100 Hz, with one gap of 20 ms and one of 30.
        let stamps = [at(0), at(10), at(20), at(40), at(50), at(80), at(90)];
        let summary = summarise(&stamps, 100.0).unwrap();
        assert_eq!((summary.frames, summary.late, summary.very_late), (6, 2, 1));
        assert!((summary.max_ms - 30.0).abs() < 0.01);
    }
}
