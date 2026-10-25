//! CEF offscreen-rendering (OSR) backend for the Rust CEF spike.
//!
//! Two halves:
//!
//! - **Always compiled, CEF-free** (unit-tested on every platform): the
//!   decisions the backend makes, as pure functions. [`config`] (startup
//!   options, switches, framework paths), [`coords`] (page CSS -> texel
//!   mapping, popup placement), [`translate`] (core `InputEvent` -> exact `CefBrowserHost` calls),
//!   [`pool`] (the per-page shared-texture cap), [`page`] (view and popup
//!   geometry), [`cpu_frame`] (`OnPaint` copies), [`dom_query`] (the
//!   devtools messages that ask a page about its elements, and their answers)
//!   and [`devtools_route`] (whose a devtools message is).
//! - **Feature `cef`**: `CefPageSource`, a [`specular_core::PageSource`] on
//!   windowless CEF browsers, and `run_subprocess_if_needed`:
//!   - `OnAcceleratedPaint` (macOS IOSurface) -> [`specular_core::PageFrame::GpuShared`],
//!     the representative zero-copy path; `OnPaint` -> `PageFrame::Cpu` elsewhere.
//!   - `OnPopupShow` / `OnPopupSize` + `PET_POPUP` paints -> popup layers, which
//!     Electron OSR never delivers (ADR 0038's `<select>` gap).
//!   - `ImeSetComposition` / `ImeCommitText` -> real composition, not whole-commit.
//!
//! The feature `cef-dox` type-checks the CEF code without downloading CEF
//! (`cargo clippy -p specular-cef --features cef-dox`); it cannot link or run.
//! See this crate's `README.md` for macOS bundling and the list of CEF calls
//! that have only been type-checked.

// Without `cef` the CEF-free helpers are compiled for their unit tests alone,
// so most of them have no caller.
#![cfg_attr(not(feature = "cef"), allow(dead_code))]

mod attach_query;
mod config;
mod coords;
mod cpu_frame;
mod devtools_route;
mod dom_query;
mod inspect_query;
mod key_message;
mod page;
mod pool;
mod sync_query;
mod translate;

#[cfg(all(feature = "cef", target_os = "macos"))]
mod app_protocol;
#[cfg(feature = "cef")]
mod attach_host;
#[cfg(feature = "cef")]
mod client;
#[cfg(feature = "cef")]
mod devtools;
#[cfg(feature = "cef")]
mod error;
#[cfg(feature = "cef")]
mod host_call;
#[cfg(all(feature = "cef", target_os = "macos"))]
mod iosurface;
#[cfg(feature = "cef")]
mod paint;
#[cfg(feature = "cef")]
mod process;
#[cfg(all(feature = "cef", target_os = "macos"))]
mod pump_timer;
#[cfg(feature = "cef")]
mod source;
#[cfg(feature = "cef")]
mod sync_host;
#[cfg(feature = "cef")]
mod trace;

pub use config::{CefConfig, Pump};
#[cfg(feature = "cef")]
pub use error::CefError;
#[cfg(feature = "cef")]
pub use process::run_subprocess_if_needed;
#[cfg(feature = "cef")]
pub use source::CefPageSource;
