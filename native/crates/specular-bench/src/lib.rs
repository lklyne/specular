//! Pan/zoom benchmark for the Rust CEF spike, mirroring the Electron app's
//! `POST /perf/pan-zoom/run` so the two shells are measured the same way.
//!
//! - [`profile`]: the six gesture profiles, a port of
//!   `src/shared/pan-zoom-perf-test.ts`, expanded into deterministic steps.
//! - [`stats`], [`recorder`]: presented-frame intervals reduced to the ADR
//!   0038 lab's fields (`drawFps`, `meanFrameMs`, `maxFrameMs`, `longFrames`)
//!   plus percentiles.
//! - [`textures`]: shared-texture pool counters with `PageHostStats` names,
//!   read from Electron's `/perf/page-hosts`.
//! - [`latency`]: event timestamp to the first presented frame reflecting it.
//! - [`memory`]: RSS summed over a shell's whole process tree.
//! - [`report`]: the results file; [`electron_trace`] produces one from an
//!   Electron trace, and [`compare`] renders two as a markdown table.

pub mod compare;
pub mod electron_trace;
mod error;
pub mod latency;
pub mod memory;
pub mod profile;
pub mod recorder;
pub mod report;
pub mod stats;
pub mod textures;

pub use compare::{LoadedRun, compare_markdown};
pub use error::BenchError;
pub use latency::{InputSeq, LatencySummary, LatencyTracker};
pub use memory::{MemoryReport, MemorySample, PeakSampler, sample_process_tree};
pub use profile::{
    GestureProfile, GestureStep, PHASE_GAP, PROFILES, ProfileId, STEP_INTERVAL, build_steps,
    select_profiles,
};
pub use recorder::{PhaseRecorder, PresentedFrame};
pub use report::{PhaseReport, RunReport, Shell};
pub use stats::{FrameSummary, FrameTimes};
pub use textures::{PageHostsSnapshot, TextureStats};
