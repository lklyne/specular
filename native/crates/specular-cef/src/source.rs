//! The CEF-backed [`PageSource`].

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cef::{
    Browser, BrowserHost, BrowserSettings, CefString, ImplBrowser, ImplBrowserHost, ImplFrame,
    PaintElementType, RuntimeStyle, Settings, WindowInfo,
};
use glam::Vec2;
use specular_core::{
    CssRect, CssSize, DevtoolsSink, InputEvent, LocatorBundle, PageColorScheme, PageEvent, PageId,
    PageNav, PageSource, PageSourceError, PageSpec, PointKind, validate_texture_scale,
    validate_viewport,
};

use crate::client::{new_app, new_client};
use crate::config::{CefConfig, Pump, browser_switches, windowless_frame_rate};
use crate::devtools::{Asked, Devtools, SinkSlot};
use crate::error::CefError;
use crate::host_call::dispatch;
use crate::page::{PageContext, PageGeometry, clear_events, drain_events, lock_geometry};
use crate::pool::OutstandingFrames;
use crate::process::{backend_error, declare_api_version};
#[cfg(target_os = "macos")]
use crate::pump_timer::PumpTimer;
use crate::sync_host::Capture;
use crate::translate::InputTranslator;
use crate::{dom_query, inspect_query};

/// Pages paint opaque white under transparent content, like an Electron
/// `BrowserWindow`, so CPU and GPU frames composite identically.
const OPAQUE_WHITE: u32 = 0xFFFF_FFFF;

/// How long shutdown waits for browsers to close.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) struct PageEntry {
    /// Kept for as long as the page is hosted; navigation goes through it.
    browser: Browser,
    pub(crate) host: BrowserHost,
    /// The channel the page's elements are asked about on.
    pub(crate) devtools: Devtools,
    geometry: Arc<Mutex<PageGeometry>>,
    input: InputTranslator,
    /// Whether the browser is shown (`WasHidden(false)`).
    painting: bool,
    /// The rate the page is owed, applied whenever it is shown.
    frame_rate: i32,
}

impl PageEntry {
    /// Makes CEF re-read `GetScreenInfo` and repaint at the backing size the
    /// geometry now asks for.
    fn apply_geometry(&self) {
        self.host.notify_screen_info_changed();
        self.host.was_resized();
    }
}

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
    pages: HashMap<PageId, PageEntry>,
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

fn cef_path(path: &std::path::Path) -> Result<CefString, CefError> {
    path.to_str()
        .map(CefString::from)
        .ok_or_else(|| CefError::InvalidPath(path.to_path_buf()))
}

impl CefPageSource {
    /// Loads the framework (macOS) and initialises CEF in windowless,
    /// external-message-pump mode, so the winit loop drives it through
    /// [`PageSource::pump`]. Call [`run_subprocess_if_needed`](crate::run_subprocess_if_needed)
    /// first in `main`. On macOS, create winit's `EventLoop` before this:
    /// winit must install its `NSApplication` subclass before CEF touches
    /// `NSApp`, or winit panics; this then adds `CefAppProtocol` to it.
    pub fn new(config: CefConfig) -> Result<Self, CefError> {
        #[cfg(target_os = "macos")]
        {
            crate::process::load_framework(false)?;
            crate::app_protocol::install()?;
        }
        declare_api_version();

        // CEF allows one process per root cache path: a second launch on
        // the same root hands itself to the first and exits. With no profile
        // to keep, each process gets a root of its own.
        let scratch_root = (config.cache_path.is_none())
            .then(|| std::env::temp_dir().join(format!("specular-cef-{}", std::process::id())));
        let root = (config.cache_path.as_deref().or(scratch_root.as_deref()))
            .map(cef_path)
            .transpose()?;
        // An empty cache path under the root keeps the profile in memory.
        let cache = config.cache_path.as_deref().map(cef_path).transpose()?;
        let settings = Settings {
            // The helper does not initialise the macOS sandbox
            // (`cef::sandbox::Sandbox`); measurement runs do not need it.
            no_sandbox: 1,
            windowless_rendering_enabled: 1,
            external_message_pump: 1,
            multi_threaded_message_loop: 0,
            remote_debugging_port: config.settings_debugging_port(),
            root_cache_path: root.unwrap_or_default(),
            cache_path: cache.unwrap_or_default(),
            background_color: OPAQUE_WHITE,
            ..Settings::default()
        };
        let args = cef::args::Args::new();
        let mut app = new_app(browser_switches(&config));
        let ok = cef::initialize(
            Some(args.as_main_args()),
            Some(&settings),
            Some(&mut app),
            std::ptr::null_mut(),
        );
        if ok != 1 {
            return Err(CefError::Initialize);
        }
        Ok(Self {
            pages: HashMap::new(),
            devtools_sink: SinkSlot::default(),
            capture: None,
            next_id: 0,
            focused: None,
            alive: Arc::new(AtomicUsize::new(0)),
            ui_thread: std::thread::current().id(),
            running: true,
            #[cfg(target_os = "macos")]
            pump_timer: match config.pump {
                Pump::RunLoopTimer => Some(PumpTimer::start()?),
                Pump::Caller => None,
            },
            #[cfg(target_os = "macos")]
            close_deadline: None,
            config,
            scratch_root,
        })
    }

    fn close_all(&mut self) {
        for (_, entry) in self.pages.drain() {
            entry.host.close_browser(1);
        }
        self.focused = None;
    }

    fn warn_if_browsers_open(&self) {
        let open = self.alive.load(Ordering::Acquire);
        if open > 0 {
            tracing::warn!(open, "browsers still open at CEF shutdown");
        }
    }

    fn entry_mut(&mut self, page: PageId) -> Result<&mut PageEntry, PageSourceError> {
        self.pages
            .get_mut(&page)
            .ok_or(PageSourceError::UnknownPage(page))
    }

    pub(crate) fn entry(&self, page: PageId) -> Result<&PageEntry, PageSourceError> {
        self.pages
            .get(&page)
            .ok_or(PageSourceError::UnknownPage(page))
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
        spec.validate()?;
        self.next_id += 1;
        let id = PageId(self.next_id);
        let geometry = Arc::new(Mutex::new(PageGeometry::new(spec)));
        let ctx = PageContext {
            id,
            geometry: Arc::clone(&geometry),
            frames: OutstandingFrames::default(),
            alive: Arc::clone(&self.alive),
            ui_thread: self.ui_thread,
        };
        let mut client = new_client(&ctx);
        let window_info = WindowInfo {
            windowless_rendering_enabled: 1,
            shared_texture_enabled: i32::from(self.config.uses_shared_texture()),
            external_begin_frame_enabled: 0,
            runtime_style: RuntimeStyle::ALLOY,
            ..WindowInfo::default()
        };
        let browser_settings = BrowserSettings {
            windowless_frame_rate: windowless_frame_rate(spec.frame_rate),
            background_color: OPAQUE_WHITE,
            ..BrowserSettings::default()
        };
        let create_error = || {
            backend_error(CefError::CreateBrowser {
                url: spec.url.clone(),
            })
        };
        let browser = cef::browser_host_create_browser_sync(
            Some(&window_info),
            Some(&mut client),
            Some(&CefString::from(spec.url.as_str())),
            Some(&browser_settings),
            None,
            None,
        )
        .ok_or_else(create_error)?;
        let host = browser.host().ok_or_else(create_error)?;
        self.alive.fetch_add(1, Ordering::AcqRel);
        let devtools = Devtools::attach(&host, &ctx, &self.devtools_sink);
        // The target id names the page to outside CDP clients; without a
        // debugging port nobody can use it.
        if self.devtools_port().is_some() {
            devtools.send(&host, Asked::Target, dom_query::target_info_message);
        }
        self.pages.insert(
            id,
            PageEntry {
                browser,
                host,
                devtools,
                geometry,
                input: InputTranslator::new(),
                painting: true,
                frame_rate: windowless_frame_rate(spec.frame_rate),
            },
        );
        Ok(id)
    }

    fn set_viewport(&mut self, page: PageId, viewport: CssSize) -> Result<(), PageSourceError> {
        validate_viewport(viewport)?;
        let entry = self.entry(page)?;
        lock_geometry(&entry.geometry).viewport = viewport;
        if entry.painting {
            entry.host.was_resized();
        }
        Ok(())
    }

    fn set_texture_scale(&mut self, page: PageId, scale: f32) -> Result<(), PageSourceError> {
        validate_texture_scale(scale)?;
        let entry = self.entry(page)?;
        lock_geometry(&entry.geometry).scale = scale;
        // A hidden browser picks the scale up when it is shown: resizing it
        // while hidden leaves it without frames after the show.
        if entry.painting {
            entry.apply_geometry();
        }
        Ok(())
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
        entry.host.was_hidden(i32::from(!painting));
        if painting {
            // Showing a windowless browser schedules no frame of its own, and
            // it comes back at the scale and rate it had when hidden.
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
            dispatch(&entry.host, &call);
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
        self.poll_capture();
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

/// Stops CEF. Every browser should be closed first.
pub(crate) fn stop_cef() {
    // Queued frames hold IOSurfaces; release them while CEF still runs.
    clear_events();
    cef::shutdown();
    #[cfg(target_os = "macos")]
    crate::process::unload_framework();
}

impl Drop for CefPageSource {
    fn drop(&mut self) {
        self.shutdown();
        if let Some(root) = self.scratch_root.take()
            && let Err(error) = std::fs::remove_dir_all(&root)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(root = %root.display(), "scratch cache folder not removed: {error}");
        }
    }
}
