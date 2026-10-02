//! Frame-timing hook for benchmarks and live diagnostics.
//!
//! The shell calls a [`FrameObserver`] once per presented window frame; the
//! bench crate implements it to collect frame intervals and latency, so the
//! compositor and app stay free of statistics code.

use std::time::{Duration, Instant};

use crate::scene::RenderStats;

/// Timing for one presented window frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameSample {
    /// When the frame was handed to the presentation engine.
    pub presented_at: Instant,
    /// Time since the previous presented frame (`None` for the first).
    pub interval: Option<Duration>,
    /// What the compositor drew.
    pub stats: RenderStats,
    /// Input-to-paint latency, when a forwarded input event's repaint first
    /// reached the screen in this frame.
    pub input_to_present: Option<Duration>,
}

/// Receives a [`FrameSample`] per presented frame.
pub trait FrameObserver {
    /// Called after each present, on the main thread.
    fn on_frame(&mut self, sample: &FrameSample);
}
