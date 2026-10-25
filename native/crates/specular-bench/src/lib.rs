//! Pan/zoom benchmark for the Rust app. The gesture profiles and the result
//! fields follow the Electron app's `POST /perf/pan-zoom/run`.
//!
//! - [`profile`]: the six gesture profiles, a port of
//!   `src/shared/pan-zoom-perf-test.ts`, expanded into deterministic steps.
//! - [`stats`], [`recorder`]: presented-frame intervals reduced to the ADR
//!   0038 lab's fields (`drawFps`, `meanFrameMs`, `maxFrameMs`, `longFrames`)
//!   plus percentiles.
//! - [`work`]: each frame's time step by step (update, view, cull, shaping,
//!   batching, tessellation, glyphs, upload, submit) and what it drew.
//! - [`textures`]: shared-texture pool counters with `PageHostStats` names.
//! - [`latency`]: event timestamp to the first presented frame reflecting it.
//! - [`memory`]: physical footprint (macOS) and RSS summed over a shell's
//!   whole process tree.
//! - [`breakdown`]: the same tree one row a process, named by what each is.
//! - [`bench_line`]: the JSON lines `specular` prints, read by
//!   `assemble`.
//! - [`report`]: the results file `assemble` produces.

pub(crate) mod bench_line;
pub(crate) mod breakdown;
mod cpu;
mod error;
mod footprint;
mod latency;
mod memory;
mod profile;
mod recorder;
mod report;
pub mod resize_report;
mod stats;
mod textures;
mod work;

pub use bench_line::{BenchLine, InputLatencyLine, ProfileLine};
pub use breakdown::{ProcessMemory, sample_breakdown};
pub use cpu::process_cpu_time;
pub use error::BenchError;
pub use latency::{InputSeq, LatencySummary, LatencyTracker};
pub use memory::{MemoryReport, MemorySample, PeakSampler, sample_process_tree};
pub use profile::{
    GestureProfile, GestureStep, PHASE_GAP, ProfileId, STEP_INTERVAL, build_steps, select_profiles,
};
pub use recorder::{PhaseRecorder, PresentedFrame};
pub use report::{PaintPolicy, PhaseReport, RunReport};
pub use stats::FrameSummary;
pub(crate) use stats::FrameTimes;
pub use textures::TextureStats;
pub use work::{FrameWork, StepStat, WorkRecorder, WorkReport};
