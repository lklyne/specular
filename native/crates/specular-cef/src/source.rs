//! The CEF-backed [`PageSource`].

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cef::{
    Browser, BrowserHost, BrowserSettings, CefString, ImplBrowser, ImplBrowserHost,
    KeyEvent as CefKeyEvent, KeyEventType, MouseButtonType, MouseEvent, RuntimeStyle, Settings,
    WindowInfo,
};
use specular_core::{
    CssSize, InputEvent, KeyEventKind, PageEvent, PageId, PageSource, PageSourceError, PageSpec,
    PointerButton, validate_texture_scale, validate_viewport,
};

use crate::client::{new_app, new_client};
use crate::config::{CefConfig, browser_switches, windowless_frame_rate};
use crate::error::CefError;
use crate::page::{PageContext, PageGeometry, clear_events, drain_events, lock_geometry};
use crate::pool::OutstandingFrames;
use crate::process::{backend_error, declare_api_version};
#[cfg(target_os = "macos")]
use crate::pump_timer::PumpTimer;
use crate::translate::{CefRange, HostCall, InputTranslator};

/// Pages paint opaque white under transparent content, like an Electron
/// `BrowserWindow`, so CPU and GPU frames composite identically.
const OPAQUE_WHITE: u32 = 0xFFFF_FFFF;

/// How long shutdown waits for browsers to close.
const CLOSE_TIMEOUT: Duration = Duration::from_secs(2);

struct PageEntry {
    /// Keeps the browser alive for as long as the page is hosted.
    _browser: Browser,
    host: BrowserHost,
    geometry: Arc<Mutex<PageGeometry>>,
    input: InputTranslator,
}

/// Windowless CEF browsers as a [`PageSource`]; see the crate docs.
///
/// One browser per page, sized to the page's CSS viewport at its device
/// scale factor. CEF runs single-threaded on the caller's (main) thread,
/// driven by [`PageSource::pump`] from the window event loop (macOS: by a
/// main-run-loop timer instead, see `pump_timer`).
pub struct CefPageSource {
    config: CefConfig,
    pages: HashMap<PageId, PageEntry>,
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

        let cache = config.cache_path.as_deref().map(cef_path).transpose()?;
        let settings = Settings {
            // The helper does not initialise the macOS sandbox
            // (`cef::sandbox::Sandbox`); measurement runs do not need it.
            no_sandbox: 1,
            windowless_rendering_enabled: 1,
            external_message_pump: 1,
            multi_threaded_message_loop: 0,
            remote_debugging_port: config.settings_debugging_port(),
            root_cache_path: cache.clone().unwrap_or_default(),
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
            config,
            pages: HashMap::new(),
            next_id: 0,
            focused: None,
            alive: Arc::new(AtomicUsize::new(0)),
            ui_thread: std::thread::current().id(),
            running: true,
            #[cfg(target_os = "macos")]
            pump_timer: Some(PumpTimer::start()?),
            #[cfg(target_os = "macos")]
            close_deadline: None,
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

    fn entry(&self, page: PageId) -> Result<&PageEntry, PageSourceError> {
        self.pages
            .get(&page)
            .ok_or(PageSourceError::UnknownPage(page))
    }
}

fn mouse_event(x: i32, y: i32, flags: u32) -> MouseEvent {
    MouseEvent {
        x,
        y,
        modifiers: flags,
    }
}

fn mouse_button(button: PointerButton) -> MouseButtonType {
    match button {
        PointerButton::Left => MouseButtonType::LEFT,
        PointerButton::Middle => MouseButtonType::MIDDLE,
        PointerButton::Right => MouseButtonType::RIGHT,
    }
}

fn key_event_type(kind: KeyEventKind) -> KeyEventType {
    match kind {
        KeyEventKind::RawDown => KeyEventType::RAWKEYDOWN,
        KeyEventKind::Up => KeyEventType::KEYUP,
        KeyEventKind::Char => KeyEventType::CHAR,
    }
}

fn cef_range(range: CefRange) -> cef::Range {
    cef::Range {
        from: range.from,
        to: range.to,
    }
}

/// Performs one translated call; a field-for-field copy by design.
fn dispatch(host: &BrowserHost, call: &HostCall<'_>) {
    match *call {
        HostCall::MouseMove { x, y, flags, leave } => {
            host.send_mouse_move_event(Some(&mouse_event(x, y, flags)), i32::from(leave));
        }
        HostCall::MouseClick {
            x,
            y,
            flags,
            button,
            up,
            click_count,
        } => host.send_mouse_click_event(
            Some(&mouse_event(x, y, flags)),
            mouse_button(button),
            i32::from(up),
            click_count,
        ),
        HostCall::MouseWheel {
            x,
            y,
            flags,
            delta_x,
            delta_y,
        } => host.send_mouse_wheel_event(Some(&mouse_event(x, y, flags)), delta_x, delta_y),
        HostCall::Key {
            kind,
            flags,
            windows_key_code,
            native_key_code,
            character,
        } => host.send_key_event(Some(&CefKeyEvent {
            type_: key_event_type(kind),
            modifiers: flags,
            windows_key_code,
            native_key_code,
            character,
            unmodified_character: character,
            ..CefKeyEvent::default()
        })),
        HostCall::ImeSetComposition {
            text,
            replacement,
            selection,
        } => host.ime_set_composition(
            Some(&CefString::from(text)),
            None,
            Some(&cef_range(replacement)),
            Some(&cef_range(selection)),
        ),
        HostCall::ImeCommit { text, replacement } => {
            host.ime_commit_text(
                Some(&CefString::from(text)),
                Some(&cef_range(replacement)),
                0,
            );
        }
        HostCall::ImeFinish { keep_selection } => {
            host.ime_finish_composing_text(i32::from(keep_selection));
        }
        HostCall::ImeCancel => host.ime_cancel_composition(),
    }
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
        self.pages.insert(
            id,
            PageEntry {
                _browser: browser,
                host,
                geometry,
                input: InputTranslator::new(),
            },
        );
        Ok(id)
    }

    fn set_viewport(&mut self, page: PageId, viewport: CssSize) -> Result<(), PageSourceError> {
        validate_viewport(viewport)?;
        let entry = self.entry(page)?;
        lock_geometry(&entry.geometry).viewport = viewport;
        entry.host.was_resized();
        Ok(())
    }

    fn set_texture_scale(&mut self, page: PageId, scale: f32) -> Result<(), PageSourceError> {
        validate_texture_scale(scale)?;
        let entry = self.entry(page)?;
        lock_geometry(&entry.geometry).scale = scale;
        // CEF re-reads GetScreenInfo on NotifyScreenInfoChanged; WasResized
        // makes it repaint at the new backing size.
        entry.host.notify_screen_info_changed();
        entry.host.was_resized();
        Ok(())
    }

    fn set_frame_rate(&mut self, page: PageId, fps: u32) -> Result<(), PageSourceError> {
        self.entry(page)?
            .host
            .set_windowless_frame_rate(windowless_frame_rate(fps));
        Ok(())
    }

    fn set_painting(&mut self, page: PageId, painting: bool) -> Result<(), PageSourceError> {
        self.entry(page)?.host.was_hidden(i32::from(!painting));
        Ok(())
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

    fn pump(&mut self) {
        // macOS pumps from `pump_timer`: this is called inside a winit
        // handler, where CEF's nested run-loop turn would re-enter winit.
        #[cfg(not(target_os = "macos"))]
        if self.running {
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
    }
}
