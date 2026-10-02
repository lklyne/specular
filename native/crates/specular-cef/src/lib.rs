//! CEF offscreen-rendering (OSR) backend for the Rust CEF spike.
//!
//! With the `cef` feature off (the default, and the only mode the Linux CI
//! job and the dev container can build) this crate exports nothing but
//! [`CEF_ENABLED`]. With it on, [`CefPageSource`] implements
//! [`specular_core::PageSource`] on windowless CEF browsers:
//!
//! - `OnAcceleratedPaint` (macOS IOSurface) -> [`specular_core::PageFrame::GpuShared`],
//!   the representative zero-copy path; `OnPaint` -> `PageFrame::Cpu` elsewhere.
//! - `OnPopupShow` / `OnPopupSize` + `PET_POPUP` paints -> popup layers, which
//!   Electron OSR never delivers (ADR 0038's `<select>` gap).
//! - `ImeSetComposition` / `ImeCommitText` -> real composition, not whole-commit.
//!
//! The feature `cef-dox` type-checks the CEF code without downloading CEF
//! (`cargo clippy -p specular-cef --features cef-dox`); it cannot link or run.

/// Whether this build hosts pages in CEF.
pub const CEF_ENABLED: bool = cfg!(feature = "cef");

#[cfg(feature = "cef")]
mod error;
#[cfg(feature = "cef")]
mod source;

#[cfg(feature = "cef")]
pub use error::CefError;
#[cfg(feature = "cef")]
pub use source::{CefConfig, CefPageSource, run_subprocess_if_needed};
