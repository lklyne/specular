//! Scripted pan/zoom gestures and frame-timing statistics for the spike.
//!
//! [`profile`] is a line-for-line port of the Electron app's
//! `src/shared/pan-zoom-perf-test.ts`, so the Rust shell and
//! `POST /perf/pan-zoom/run` drive the camera through identical input.
//! [`stats`] turns measured frame intervals into the same fields the Electron
//! Offscreen Rendering Lab reported in ADR 0038 (`drawFps`, `meanFrameMs`,
//! `maxFrameMs`, `longFrames`).

pub mod profile;
pub mod stats;

pub use profile::{
    GestureProfile, GestureStep, PHASE_GAP, PROFILES, ProfileId, STEP_INTERVAL, build_steps,
};
pub use stats::{FrameSummary, FrameTimes};
