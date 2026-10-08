//! CEF backend errors.

use std::io;
use std::path::PathBuf;

/// Errors starting or driving CEF.
#[derive(Debug, thiserror::Error)]
pub enum CefError {
    /// The CEF framework could not be found (macOS: not inside an app bundle
    /// with `Chromium Embedded Framework.framework`; see the crate README).
    #[error(
        "cannot find the CEF framework at {} (run from the .app bundle, see \
         crates/specular-cef/README.md)",
        path.display()
    )]
    FrameworkNotFound {
        /// The framework library path looked up.
        path: PathBuf,
        /// Why.
        #[source]
        source: io::Error,
    },
    /// The running executable's path is unknown, so the framework (which
    /// sits relative to it) cannot be located.
    #[error("cannot locate the running executable")]
    CurrentExe(#[source] io::Error),
    /// No framework path can be formed from the executable's location.
    #[error("cannot derive a CEF framework path from {}", .0.display())]
    FrameworkPath(PathBuf),
    /// `cef_load_library` refused the framework.
    #[error("cef_load_library failed for {}", .0.display())]
    LoadLibrary(PathBuf),
    /// `NSApp` could not be made to implement `CefAppProtocol` (macOS).
    #[error("cannot make NSApp a CefAppProtocol application: {0}")]
    AppProtocol(&'static str),
    /// The main run-loop timer that pumps CEF could not be created (macOS).
    #[error("cannot start the CEF message pump timer")]
    PumpTimer,
    /// A configured path cannot be passed to CEF (not valid UTF-8).
    #[error("path is not valid UTF-8: {}", .0.display())]
    InvalidPath(PathBuf),
    /// `cef_initialize` returned failure.
    #[error("CEF failed to initialize")]
    Initialize,
    /// A windowless browser could not be created.
    #[error("failed to create browser for {url}")]
    CreateBrowser {
        /// The URL that was being opened.
        url: String,
    },
    /// The page has no main frame to load a URL into.
    #[error("the page has no main frame")]
    NoMainFrame,
    /// The page's devtools channel refused a question.
    #[error("the page's DevTools channel refused: {0}")]
    Devtools(&'static str),
}
