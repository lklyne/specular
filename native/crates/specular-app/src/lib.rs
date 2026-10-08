//! What a Specular shell is made of, whatever opens its window.
//!
//! [`Runtime`] owns the [`App`](specular_interact::App) and runs every
//! effect `update` returns: page hosts, the files of the space, images,
//! Documents, the clipboard, assets, settings and the HTTP API. A shell
//! gives it a [`ShellWindow`] to draw into and sends it events.
//! [`launch`] reads the command line and runs a `--snapshot` or `--script`
//! request with no window at all.
//!
//! [`run_window`] is the winit shell, the `specular-app` binary. The GPUI
//! Kit shell is the `specular-shell` crate (ADR 0040).

mod api;
mod app;
mod bench_run;
mod cli;
mod headless;
mod images;
mod latency;
mod launch;
mod notes;
pub mod offscreen;
mod page_notice;
mod page_queries;
mod paint_lod;
mod persist;
mod prefs;
mod scene;
mod source_select;
mod space;
mod translate;

pub use crate::app::{Opening, PageOf, Runtime, RuntimeOptions, ShellWindow};
pub use crate::launch::{Launch, launch, run_window};
pub use crate::source_select::run_subprocess_if_needed;
pub use crate::translate::native_key_input;
