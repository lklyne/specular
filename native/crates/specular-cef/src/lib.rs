//! CEF offscreen-rendering (OSR) backend for the Rust CEF spike.
//!
//! Two halves:
//!
//! - **Always compiled, CEF-free** (unit-tested on every platform): the
//!   decisions the backend makes, as pure functions. [`config`] (startup
//!   options, switches, framework paths), [`coords`] (canvas -> page CSS ->
//!   texel mapping, popup placement), [`keys`] (W3C `code` -> CEF key codes),
//!   [`translate`] (core `InputEvent` -> exact `CefBrowserHost` calls),
//!   [`pool`] (the per-page shared-texture cap), [`page`] (view and popup
//!   geometry) and [`cpu_frame`] (`OnPaint` copies).
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

/// Whether this build hosts pages in CEF.
pub const CEF_ENABLED: bool = cfg!(feature = "cef");

pub mod config;
pub mod coords;
pub mod cpu_frame;
pub mod keys;
pub mod page;
pub mod pool;
pub mod translate;

#[cfg(feature = "cef")]
mod client;
#[cfg(feature = "cef")]
mod error;
#[cfg(all(feature = "cef", target_os = "macos"))]
mod iosurface;
#[cfg(feature = "cef")]
mod paint;
#[cfg(feature = "cef")]
mod process;
#[cfg(feature = "cef")]
mod source;

pub use config::CefConfig;
#[cfg(feature = "cef")]
pub use error::CefError;
#[cfg(feature = "cef")]
pub use process::run_subprocess_if_needed;
#[cfg(feature = "cef")]
pub use source::CefPageSource;
