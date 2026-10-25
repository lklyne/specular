//! Starting CEF, and stopping it with every browser closed.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use cef::{CefString, ImplBrowserHost, Settings};
use specular_core::PageSource;

use super::{CefPageSource, OPAQUE_WHITE};
use crate::client::new_app;
use crate::config::{CefConfig, Pump, browser_switches};
use crate::devtools::SinkSlot;
use crate::error::CefError;
use crate::page::clear_events;
use crate::process::declare_api_version;
#[cfg(target_os = "macos")]
use crate::pump_timer::PumpTimer;

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

    pub(super) fn close_all(&mut self) {
        for (_, entry) in self.pages.drain() {
            entry.host.close_browser(1);
        }
        self.focused = None;
    }

    pub(super) fn warn_if_browsers_open(&self) {
        let open = self.alive.load(Ordering::Acquire);
        if open > 0 {
            tracing::warn!(open, "browsers still open at CEF shutdown");
        }
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
