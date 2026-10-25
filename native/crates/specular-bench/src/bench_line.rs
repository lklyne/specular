//! The JSON lines `specular-app` prints on stdout, which `assemble` folds
//! into a [`RunReport`](crate::RunReport). One type on both sides, so a field
//! renamed in the app fails `assemble` instead of reading as zero.

use serde::{Deserialize, Serialize};

use crate::{LatencySummary, PaintPolicy, PhaseReport, WorkReport};

/// One line of `specular-app` output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
#[expect(
    clippy::large_enum_variant,
    reason = "one value per printed line, never stored in bulk"
)]
pub enum BenchLine {
    /// One gesture profile's result (`--bench`).
    Profile(ProfileLine),
    /// The session's forwarded-input latency, printed at exit when any input
    /// reached a page.
    InputLatency(InputLatencyLine),
}

/// One profile's result: the [`PhaseReport`] fields at the top level, plus
/// what only the Rust shell knows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileLine {
    /// Frame timing and texture counters.
    #[serde(flatten)]
    pub phase: PhaseReport,
    /// The profile's human-readable label.
    pub label: String,
    /// Page source name (`cef`, `synthetic`).
    pub source: String,
    /// Pages on the canvas.
    pub pages: usize,
    /// False when the source is synthetic or any frame drawn came from a CPU
    /// upload, which does not measure the real page cost (ADR 0038).
    pub representative: bool,
    /// Refresh interval the profile was stepped at.
    pub step_interval_ms: f64,
    /// Longest wait between a paint arriving and the frame showing it.
    pub max_paint_to_submit_ms: Option<f64>,
    /// The page paint policy the shell applied (frame-rate tiers, culling),
    /// so `compare` can flag runs whose shells did different work.
    pub paint_policy: PaintPolicy,
    /// Whether the chrome layer was drawn every frame. Older lines predate
    /// it and drew none.
    #[serde(default)]
    pub chrome: bool,
    /// Annotations drawn every frame; zero in older lines.
    #[serde(default)]
    pub annotations: usize,
    /// What the frames were drawn into: `window`, or `headless` for a
    /// texture with no vsync. Older lines were all windows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The shell whose window presented the frames: `winit` or `kit`.
    /// Older lines and headless ones name none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    /// The canvas the run showed, by file name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canvas: Option<String>,
    /// Each frame's time step by step. Older lines have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work: Option<WorkReport>,
}

/// Forwarded input -> page repaint -> presented, over a whole session.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputLatencyLine {
    /// The latency distribution.
    pub input_latency: LatencySummary,
}
