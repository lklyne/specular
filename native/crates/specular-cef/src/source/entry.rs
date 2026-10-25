//! One hosted page: its browser, and what it is owed when shown.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use cef::{
    Browser, BrowserHost, BrowserSettings, CefString, ImplBrowser, ImplBrowserHost, ImplFrame,
    RuntimeStyle, WindowInfo,
};
use specular_core::{PageDevice, PageId, PageSource, PageSourceError, PageSpec};

use super::{CefPageSource, OPAQUE_WHITE};
use crate::attach_host::Tracking;
use crate::client::new_client;
use crate::config::windowless_frame_rate;
use crate::devtools::{Asked, Devtools, Pending};
use crate::dom_query;
use crate::error::CefError;
use crate::page::{GeometryChange, PageContext, PageGeometry, Unsent, lock_geometry};
use crate::pool::OutstandingFrames;
use crate::process::backend_error;
use crate::translate::InputTranslator;

pub(crate) struct PageEntry {
    /// Kept for as long as the page is hosted; navigation goes through it.
    pub(super) browser: Browser,
    pub(crate) host: BrowserHost,
    /// The channel the page's elements are asked about on.
    pub(crate) devtools: Devtools,
    pub(super) geometry: Arc<Mutex<PageGeometry>>,
    /// The device the page is told it is on at the start of every load.
    pub(super) device: Arc<Mutex<PageDevice>>,
    pub(super) input: InputTranslator,
    /// Whether the browser is shown (`WasHidden(false)`).
    pub(super) painting: bool,
    /// The geometry change the host has not been told of. Always empty for
    /// a hidden page, which is told when it is shown.
    pub(super) unsent: Unsent,
    /// The rate the page is owed, applied whenever it is shown.
    pub(super) frame_rate: i32,
    /// The elements whose place the page reports.
    pub(crate) tracking: Tracking,
}

impl PageEntry {
    /// Makes CEF re-read `GetScreenInfo` and repaint at the backing size the
    /// geometry now asks for.
    pub(super) fn apply_geometry(&self) {
        self.host.notify_screen_info_changed();
        self.host.was_resized();
    }

    /// Tells the host of the geometry as it stands, as one resize.
    pub(super) fn send(&self, page: PageId, change: GeometryChange) {
        let asked = lock_geometry(&self.geometry).clone();
        crate::trace::asked(page, &asked, change.name());
        match change {
            GeometryChange::Viewport => self.host.was_resized(),
            GeometryChange::Scale | GeometryChange::Both => self.apply_geometry(),
        }
    }
}

impl CefPageSource {
    /// Opens a windowless browser on `spec` and hosts it as a new page.
    pub(super) fn open_page(&mut self, spec: &PageSpec) -> Result<PageId, PageSourceError> {
        spec.validate()?;
        self.next_id += 1;
        let id = PageId(self.next_id);
        let geometry = Arc::new(Mutex::new(PageGeometry::new(spec)));
        let device = Arc::new(Mutex::new(spec.device));
        let ctx = PageContext {
            id,
            geometry: Arc::clone(&geometry),
            frames: OutstandingFrames::default(),
            alive: Arc::clone(&self.alive),
            ui_thread: self.ui_thread,
            device: Arc::clone(&device),
            pending: Pending::default(),
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
        devtools.send_device(&host, &spec.device);
        // The load the browser was made with may have asked for its
        // document as the browser itself. Asked for again, it is the device.
        if spec.device.user_agent.is_some() {
            (browser.main_frame())
                .ok_or_else(|| backend_error(CefError::NoMainFrame))?
                .load_url(Some(&CefString::from(spec.url.as_str())));
        }
        (browser.main_frame())
            .ok_or_else(|| backend_error(CefError::NoMainFrame))?
            .load_url(Some(&CefString::from(spec.url.as_str())));
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
                device,
                input: InputTranslator::new(),
                painting: true,
                unsent: Unsent::default(),
                frame_rate: windowless_frame_rate(spec.frame_rate),
                tracking: Tracking::default(),
            },
        );
        Ok(id)
    }

    pub(crate) fn entry_mut(&mut self, page: PageId) -> Result<&mut PageEntry, PageSourceError> {
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
