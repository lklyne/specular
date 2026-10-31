//! What a Specular shell is made of, whatever opens its window.
//!
//! [`Runtime`] owns the [`App`](specular_interact::App) and runs every
//! effect `update` returns: page hosts, the files of the space, images,
//! Documents, the clipboard, assets, settings and the HTTP API. A shell
//! gives it a [`ShellWindow`] to draw into and sends it events.
//! [`launch`] reads the command line and runs a `--snapshot` or `--script`
//! request with no window at all. [`Bench`] is `--bench` in a window: a
//! shell steps it each loop turn and hands it the frames it presents.
//!
//! The window is the GPUI Kit shell, the `specular-shell` crate (ADR 0040).

pub mod agent;
#[cfg(unix)]
pub mod agent_tools;
mod api;
mod app;
mod bench_drive;
mod bench_run;
mod cdp;
mod cli;
mod crash_log;
mod headless;
mod images;
mod latency;
mod launch;
mod notes;
pub mod offscreen;
mod page_files;
mod page_notice;
mod page_queries;
mod paint_lod;
mod persist;
mod prefs;
mod scene;
pub mod script;
mod share;
mod source_select;
mod space;
mod watch;

pub use crate::app::{FolderAsk, Opening, PageOf, Runtime, RuntimeOptions, ShellWindow};
pub use crate::bench_drive::{Bench, BenchOptions};
pub use crate::headless::{Report, run_scenario};
pub use crate::launch::{Launch, launch};
pub use crate::source_select::run_subprocess_if_needed;
