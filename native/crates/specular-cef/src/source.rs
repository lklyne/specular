//! The CEF-backed [`PageSource`].

use std::path::PathBuf;

use specular_core::{
    CssSize, InputEvent, PageEvent, PageId, PageSource, PageSourceError, PageSpec,
};

use crate::error::CefError;

/// Startup options for [`CefPageSource`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CefConfig {
    /// Expose CDP on this port (`--remote-debugging-port`), for agents and
    /// for driving the bench's input-latency probes.
    pub remote_debugging_port: Option<u16>,
    /// Profile/cache directory; `None` uses an ephemeral in-memory profile.
    pub cache_path: Option<PathBuf>,
    /// Use `OnAcceleratedPaint` shared textures (`shared_texture_enabled`).
    /// Off forces `OnPaint` CPU frames, which are non-representative.
    pub shared_texture: bool,
}

/// If this process was launched by CEF as a subprocess (renderer, GPU,
/// utility), runs it to completion and returns its exit code; returns `None`
/// in the browser process. Call first thing in `main`.
pub fn run_subprocess_if_needed() -> Option<i32> {
    None
}

/// Windowless CEF browsers as a [`PageSource`]; see the crate docs.
#[derive(Debug)]
pub struct CefPageSource {
    config: CefConfig,
}

impl CefPageSource {
    /// Initialises CEF (external message pump mode, so the winit loop drives
    /// it through [`PageSource::pump`]).
    pub fn new(config: CefConfig) -> Result<Self, CefError> {
        Ok(Self { config })
    }
}

impl PageSource for CefPageSource {
    fn name(&self) -> &'static str {
        "cef"
    }

    fn create_page(&mut self, spec: &PageSpec) -> Result<PageId, PageSourceError> {
        Err(PageSourceError::InvalidSpec(format!(
            "CEF page creation not implemented yet ({})",
            spec.url
        )))
    }

    fn set_viewport(&mut self, page: PageId, _viewport: CssSize) -> Result<(), PageSourceError> {
        Err(PageSourceError::UnknownPage(page))
    }

    fn set_texture_scale(&mut self, page: PageId, _scale: f32) -> Result<(), PageSourceError> {
        Err(PageSourceError::UnknownPage(page))
    }

    fn set_frame_rate(&mut self, page: PageId, _fps: u32) -> Result<(), PageSourceError> {
        Err(PageSourceError::UnknownPage(page))
    }

    fn set_painting(&mut self, page: PageId, _painting: bool) -> Result<(), PageSourceError> {
        Err(PageSourceError::UnknownPage(page))
    }

    fn close_page(&mut self, page: PageId) -> Result<(), PageSourceError> {
        Err(PageSourceError::UnknownPage(page))
    }

    fn set_focus(&mut self, page: Option<PageId>) -> Result<(), PageSourceError> {
        page.map_or(Ok(()), |id| Err(PageSourceError::UnknownPage(id)))
    }

    fn send_input(&mut self, page: PageId, _event: &InputEvent) -> Result<(), PageSourceError> {
        Err(PageSourceError::UnknownPage(page))
    }

    fn pump(&mut self) {}

    fn drain_events(&mut self, _out: &mut Vec<PageEvent>) {}

    fn devtools_port(&self) -> Option<u16> {
        self.config.remote_debugging_port
    }

    fn shutdown(&mut self) {}
}
