//! The CEF-backed [`PageSource`].

mod entry;
mod start;

use self::entry::PageEntry;
pub(crate) use self::start::stop_cef;

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, PoisonError};
use std::time::{Duration, Instant};

use cef::{CefString, ImplBrowser, ImplBrowserHost, ImplFrame, PaintElementType};
use glam::Vec2;
use specular_core::{
    CssRect, CssSize, DevtoolsSink, InputEvent, LocatorBundle, PageColorScheme, PageDevice,
    PageEvent, PageId, PageNav, PageSource, PageSourceError, PageSpec, PointKind,
    validate_texture_scale, validate_viewport,
};

use crate::config::{CefConfig, windowless_frame_rate};
use crate::devtools::{Asked, Devtools, SinkSlot};
use crate::error::CefError;
use crate::host_call::dispatch;
use crate::page::{GeometryChange, Unsent, drain_events, lock_geometry};
use crate::process::backend_error;
#[cfg(target_os = "macos")]
use crate::pump_timer::PumpTimer;
use crate::sync_host::Capture;
use crate::{dom_query, inspect_query};

/// Pages paint opaque white under transparent content, like an Electron
/// `BrowserWindow`, so CPU and GPU frames composite identically.
const OPAQUE_WHITE: u32 = 0xFFFF_FFFF;

/// How long shutdown waits for browsers to close.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

/// Windowless CEF browsers as a [`PageSource`]; see the crate docs.
///
/// One browser per page, sized to the page's CSS viewport at its device
/// scale factor. CEF runs single-threaded on the caller's (main) thread,
/// driven by [`PageSource::pump`] from the window event loop (macOS: by a
/// main-run-loop timer instead, see `pump_timer`).
pub struct CefPageSource {
    config: CefConfig,
    /// The root cache folder made for this process alone, removed when the
    /// source is dropped.
    scratch_root: Option<std::path::PathBuf>,
    pub(crate) pages: HashMap<PageId, PageEntry>,
    /// Where every page's devtools answers and events go.
    devtools_sink: SinkSlot,
    /// The page whose hovers and clicks are reported, for interaction sync.
    pub(crate) capture: Option<Capture>,
    next_id: u64,
    focused: Option<PageId>,
    alive: Arc<AtomicUsize>,
    ui_thread: std::thread::ThreadId,
    running: bool,
    #[cfg(target_os = "macos")]
    pump_timer: Option<PumpTimer>,
    /// Set by the first `poll_shutdown`: when to stop waiting for browsers.
    #[cfg(target_os = "macos")]
    close_deadline: Option<Instant>,
}

impl fmt::Debug for CefPageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CefPageSource")
            .field("config", &self.config)
            .field("pages", &self.pages.len())
            .field("focused", &self.focused)
            .field("running", &self.running)
            .finish_non_exhaustive()
    }
}

/// A question the page's devtools channel would not take.
pub(crate) fn refused(what: &'static str) -> PageSourceError {
    backend_error(CefError::Devtools(what))
}

impl PageSource for CefPageSource {
    fn name(&self) -> &'static str {
        "cef"
    }

    fn create_page(&mut self, spec: &PageSpec) -> Result<PageId, PageSourceError> {
        self.open_page(spec)
    }

    fn set_viewport(&mut self, page: PageId, viewport: CssSize) -> Result<(), PageSourceError> {
        validate_viewport(viewport)?;
        let entry = self.entry_mut(page)?;
        lock_geometry(&entry.geometry).viewport = viewport;
        if entry.painting {
            entry.unsent.changed(GeometryChange::Viewport);
        }
        Ok(())
    }

    fn set_texture_scale(&mut self, page: PageId, scale: f32) -> Result<(), PageSourceError> {
        validate_texture_scale(scale)?;
        let entry = self.entry_mut(page)?;
        lock_geometry(&entry.geometry).scale = scale;
        // A hidden browser picks the scale up when it is shown: resizing it
        // while hidden leaves it without frames after the show.
        if entry.painting {
            entry.unsent.changed(GeometryChange::Scale);
        }
        Ok(())
    }

    fn flush_geometry(&mut self) {
        for (page, entry) in &mut self.pages {
            if let Some(change) = entry.unsent.take() {
                entry.send(*page, change);
            }
        }
    }

    fn set_frame_rate(&mut self, page: PageId, fps: u32) -> Result<(), PageSourceError> {
        let entry = self.entry_mut(page)?;
        entry.frame_rate = windowless_frame_rate(fps);
        if entry.painting {
            entry.host.set_windowless_frame_rate(entry.frame_rate);
        }
        Ok(())
    }

    fn set_painting(&mut self, page: PageId, painting: bool) -> Result<(), PageSourceError> {
        let entry = self.entry_mut(page)?;
        if entry.painting == painting {
            return Ok(());
        }
        entry.painting = painting;
        // Shown, it is told of the geometry below; hidden, when next shown.
        entry.unsent = Unsent::default();
        entry.host.was_hidden(i32::from(!painting));
        if painting {
            // Showing a windowless browser schedules no frame of its own, and
            // it comes back at the scale and rate it had when hidden.
            let asked = lock_geometry(&entry.geometry).clone();
            crate::trace::asked(page, &asked, "shown");
            entry.apply_geometry();
            entry.host.set_windowless_frame_rate(entry.frame_rate);
            entry.host.invalidate(PaintElementType::VIEW);
        }
        Ok(())
    }

    fn set_color_scheme(
        &mut self,
        page: PageId,
        scheme: PageColorScheme,
    ) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        (entry.devtools)
            .send(&entry.host, Asked::Done, |id| {
                dom_query::color_scheme_message(id, scheme)
            })
            .then_some(())
            .ok_or_else(|| refused("color scheme"))
    }

    fn set_device(&mut self, page: PageId, device: &PageDevice) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        *entry.device.lock().unwrap_or_else(PoisonError::into_inner) = *device;
        (entry.devtools)
            .send_device(&entry.host, device)
            .then_some(())
            .ok_or_else(|| refused("device"))
    }

    fn close_page(&mut self, page: PageId) -> Result<(), PageSourceError> {
        let entry = self
            .pages
            .remove(&page)
            .ok_or(PageSourceError::UnknownPage(page))?;
        if self.focused == Some(page) {
            self.focused = None;
        }
        entry.host.close_browser(1);
        Ok(())
    }

    fn set_focus(&mut self, page: Option<PageId>) -> Result<(), PageSourceError> {
        if page == self.focused {
            return Ok(());
        }
        if let Some(next) = page {
            self.entry(next)?;
        }
        if let Some(previous) = self.focused.and_then(|id| self.pages.get(&id)) {
            previous.host.set_focus(0);
        }
        if let Some(next) = page.and_then(|id| self.pages.get(&id)) {
            next.host.set_focus(1);
        }
        self.focused = page;
        Ok(())
    }

    fn send_input(&mut self, page: PageId, event: &InputEvent) -> Result<(), PageSourceError> {
        let entry = self
            .pages
            .get_mut(&page)
            .ok_or(PageSourceError::UnknownPage(page))?;
        for call in entry.input.translate(event) {
            dispatch(&entry.host, &entry.devtools, &call);
        }
        Ok(())
    }

    fn navigate(&mut self, page: PageId, nav: &PageNav) -> Result<(), PageSourceError> {
        let browser = &self.entry(page)?.browser;
        match nav {
            PageNav::To(url) => {
                let frame =
                    (browser.main_frame()).ok_or_else(|| backend_error(CefError::NoMainFrame))?;
                frame.load_url(Some(&CefString::from(url.as_str())));
            }
            PageNav::Back => browser.go_back(),
            PageNav::Forward => browser.go_forward(),
            PageNav::Reload => browser.reload(),
            PageNav::Stop => browser.stop_load(),
        }
        Ok(())
    }

    fn scroll_progress(&mut self, page: PageId) -> Result<(), PageSourceError> {
        self.ask_scroll_progress(page)
    }

    fn scroll_room(&mut self, page: PageId, at: Vec2, toward: Vec2) -> Result<(), PageSourceError> {
        self.ask_scroll_room(page, at, toward)
    }

    fn scroll_to(&mut self, page: PageId, progress: Vec2) -> Result<(), PageSourceError> {
        self.scroll_to_progress(page, progress)
    }

    fn set_capture(&mut self, page: Option<PageId>) -> Result<(), PageSourceError> {
        self.capture(page)
    }

    fn query_candidates(
        &mut self,
        page: PageId,
        bundle: &LocatorBundle,
        request: u64,
    ) -> Result<(), PageSourceError> {
        self.ask_candidates(page, bundle, request)
    }

    fn replay_pointer(
        &mut self,
        page: PageId,
        kind: PointKind,
        point: Vec2,
    ) -> Result<(), PageSourceError> {
        self.replay(page, kind, point)
    }

    fn query_element(
        &mut self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        let sent = entry
            .devtools
            .send(&entry.host, Asked::Element(request), |id| {
                dom_query::element_at_message(id, point.x, point.y)
            });
        sent.then_some(())
            .ok_or_else(|| refused("element at point"))
    }

    fn inspect_at(
        &mut self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        let sent = entry
            .devtools
            .send(&entry.host, Asked::Inspect(request), |id| {
                inspect_query::inspect_at_message(id, point.x, point.y)
            });
        sent.then_some(())
            .ok_or_else(|| refused("inspect at point"))
    }

    fn capture_element(
        &mut self,
        page: PageId,
        point: Vec2,
        request: u64,
    ) -> Result<(), PageSourceError> {
        self.ask_capture(page, point, request)
    }

    fn track_elements(
        &mut self,
        page: PageId,
        selectors: &[String],
    ) -> Result<(), PageSourceError> {
        self.track(page, selectors)
    }

    fn query_elements_in_rect(
        &mut self,
        page: PageId,
        rect: CssRect,
        request: u64,
    ) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        let sent = entry
            .devtools
            .send(&entry.host, Asked::ElementsInRect(request), |id| {
                dom_query::elements_in_rect_message(id, rect)
            });
        sent.then_some(())
            .ok_or_else(|| refused("elements in rect"))
    }

    fn pump(&mut self) {
        // A caller that never ends a turn with `flush_geometry` still gets
        // its resize, a pump late.
        for (page, entry) in &mut self.pages {
            if let Some(change) = entry.unsent.take_at_pump() {
                entry.send(*page, change);
            }
        }
        self.poll_capture();
        self.poll_tracking();
        // With a pump timer this is called inside a window event loop's
        // handler, where CEF's nested run-loop turn would re-enter the loop.
        #[cfg(target_os = "macos")]
        let timed = self.pump_timer.is_some();
        #[cfg(not(target_os = "macos"))]
        let timed = false;
        if self.running && !timed {
            cef::do_message_loop_work();
        }
    }

    fn drain_events(&mut self, out: &mut Vec<PageEvent>) {
        drain_events(out);
    }

    fn devtools_port(&self) -> Option<u16> {
        self.config
            .remote_debugging_port
            .filter(|_| self.config.settings_debugging_port() != 0)
    }

    fn devtools_send(&mut self, page: PageId, message: &str) -> Result<(), PageSourceError> {
        let entry = self.entry(page)?;
        Devtools::send_raw(&entry.host, message)
            .then_some(())
            .ok_or_else(|| refused("client message"))
    }

    fn set_devtools_sink(&mut self, sink: Option<DevtoolsSink>) {
        self.devtools_sink.set(sink);
    }

    fn shutdown(&mut self) {
        if !self.running {
            return;
        }
        #[cfg(target_os = "macos")]
        drop(self.pump_timer.take());
        self.close_all();
        let deadline = Instant::now() + CLOSE_TIMEOUT;
        while self.alive.load(Ordering::Acquire) > 0 && Instant::now() < deadline {
            cef::do_message_loop_work();
            std::thread::sleep(Duration::from_millis(1));
        }
        self.warn_if_browsers_open();
        stop_cef();
        self.running = false;
    }

    /// Closes the browsers, lets the pump timer run CEF until they are gone,
    /// then has the timer stop CEF: both the close and `cef::shutdown` spin
    /// run-loop turns, which must happen outside winit's handlers and before
    /// its event loop returns (see `pump_timer`).
    #[cfg(target_os = "macos")]
    fn poll_shutdown(&mut self) -> bool {
        if !self.running {
            return true;
        }
        let Some(deadline) = self.close_deadline else {
            self.close_all();
            self.close_deadline = Some(Instant::now() + CLOSE_TIMEOUT);
            return false;
        };
        if self.alive.load(Ordering::Acquire) > 0 && Instant::now() < deadline {
            return false;
        }
        if self.pump_timer.is_none() {
            self.shutdown();
            return true;
        }
        if PumpTimer::request_stop() {
            self.warn_if_browsers_open();
        }
        if !PumpTimer::stopped() {
            return false;
        }
        self.pump_timer = None;
        self.running = false;
        true
    }
}
