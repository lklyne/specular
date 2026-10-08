//! The run's event log and frame-pacing summaries.

use std::sync::Mutex;
use std::time::Instant;

static LINES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Appends one line to the run's log.
pub fn line(text: String) {
    if let Ok(mut lines) = LINES.lock() {
        lines.push(text);
    }
}

/// Takes every line logged so far.
pub fn take() -> Vec<String> {
    LINES
        .lock()
        .map(|mut lines| std::mem::take(&mut *lines))
        .unwrap_or_default()
}

/// Frame timestamps for one renderer.
#[derive(Default)]
pub struct Pacing {
    stamps: Vec<Instant>,
}

impl Pacing {
    pub fn mark(&mut self) {
        self.stamps.push(Instant::now());
    }

    pub fn clear(&mut self) {
        self.stamps.clear();
    }

    /// Interval statistics in milliseconds against a refresh of `hz`.
    pub fn summary(&self, name: &str, hz: f64) -> String {
        if self.stamps.len() < 3 {
            return format!("{name}: {} frames, too few to summarise", self.stamps.len());
        }
        let mut gaps: Vec<f64> = self
            .stamps
            .windows(2)
            .map(|pair| pair[1].duration_since(pair[0]).as_secs_f64() * 1000.0)
            .collect();
        let total: f64 = gaps.iter().sum();
        let frame = 1000.0 / hz;
        let late = gaps.iter().filter(|&&gap| gap > frame * 1.5).count();
        let very_late = gaps.iter().filter(|&&gap| gap > frame * 2.5).count();
        gaps.sort_by(f64::total_cmp);
        let at = |q: f64| gaps[((gaps.len() - 1) as f64 * q).round() as usize];
        format!(
            "{name}: {} frames in {:.2} s = {:.1} fps; interval ms mean {:.2} p50 {:.2} p95 {:.2} p99 {:.2} max {:.2}; over 1.5 frames: {late} ({:.2}%), over 2.5 frames: {very_late}",
            gaps.len(),
            total / 1000.0,
            gaps.len() as f64 / (total / 1000.0),
            total / gaps.len() as f64,
            at(0.5),
            at(0.95),
            at(0.99),
            at(1.0),
            100.0 * late as f64 / gaps.len() as f64,
        )
    }
}
