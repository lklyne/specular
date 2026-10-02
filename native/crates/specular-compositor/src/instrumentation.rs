//! Frame-timing hook for benchmarks and live diagnostics.
//!
//! The shell calls a [`FrameObserver`] once per presented window frame; the
//! app's bench runner implements it and hands each frame to the bench
//! crate's recorder, so the compositor stays free of statistics code.

use std::time::{Duration, Instant};

use crate::scene::RenderStats;

/// Timing for one presented window frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameSample {
    /// When the frame was handed to the presentation engine.
    pub presented_at: Instant,
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
