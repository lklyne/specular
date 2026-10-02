//! CEF backend errors.

use std::path::PathBuf;

/// Errors starting or driving CEF.
#[derive(Debug, thiserror::Error)]
pub enum CefError {
    /// The CEF framework could not be loaded (macOS: not inside an app bundle
    /// with `Chromium Embedded Framework.framework`; see the crate README).
    #[error("failed to load the CEF framework: {0}")]
    LoadFramework(String),
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
}
