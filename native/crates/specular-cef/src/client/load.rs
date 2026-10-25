//! Loads, requests and each browser's life span: navigation, new-tab links and renderer deaths.

use std::os::raw::c_int;

use cef::rc::Rc as _;
use cef::{
    Browser, BrowserSettings, CefString, Client, DictionaryValue, Frame, ImplBrowser, ImplFrame,
    ImplLifeSpanHandler, ImplLoadHandler, ImplRequestHandler, LifeSpanHandler, LoadHandler,
    PopupFeatures, RequestHandler, TerminationStatus, TransitionType, WindowInfo,
    WindowOpenDisposition, WrapLifeSpanHandler, WrapLoadHandler, WrapRequestHandler,
    wrap_life_span_handler, wrap_load_handler, wrap_request_handler,
};
use specular_core::PageEvent;

use crate::page::PageContext;

wrap_load_handler! {
    pub(super) struct PageLoadHandler {
        ctx: PageContext,
    }

    impl LoadHandler {
        fn on_loading_state_change(
            &self,
            _browser: Option<&mut Browser>,
            is_loading: c_int,
            can_go_back: c_int,
            can_go_forward: c_int,
        ) {
            self.ctx.push(PageEvent::Loading {
                page: self.ctx.id,
                loading: is_loading != 0,
                can_go_back: can_go_back != 0,
                can_go_forward: can_go_forward != 0,
            });
        }

        fn on_load_start(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            _transition_type: TransitionType,
        ) {
            // A document in a new renderer is not told what the last one
            // was, so `navigator.platform` would be the browser's again.
            if frame.is_some_and(|frame| frame.is_main() != 0)
                && let Some(host) = browser.and_then(|browser| browser.host())
            {
                self.ctx.pending.send_device(&host, &self.ctx.device());
            }
        }

        fn on_load_end(
            &self,
            browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            http_status_code: c_int,
        ) {
            if frame.is_some_and(|frame| frame.is_main() != 0) {
                // Its styles are in, so its colour is the one it keeps.
                if let Some(host) = browser.and_then(|browser| browser.host()) {
                    self.ctx.pending.ask_viewport_fit(&host);
                }
                self.ctx.push(PageEvent::Loaded {
                    page: self.ctx.id,
                    http_status: http_status_code,
                });
            }
        }
    }
}

/// The reason string Electron's `render-process-gone` uses for a status, so
/// bench logs from both shells read the same.
fn termination_reason(status: TerminationStatus) -> &'static str {
    use cef::sys::cef_termination_status_t as Status;
    match *status.as_ref() {
        Status::TS_PROCESS_WAS_KILLED => "killed",
        Status::TS_PROCESS_CRASHED => "crashed",
        Status::TS_PROCESS_OOM => "oom",
        Status::TS_LAUNCH_FAILED => "launch-failed",
        Status::TS_INTEGRITY_FAILURE => "integrity-failure",
        _ => "abnormal-exit",
    }
}

/// Whether a link opened with `disposition` goes to a new page, and if so
/// whether that page comes to the front. A page here never gets a window of
/// its own: off-screen, it would sit outside the canvas. So a popup with
/// window features and a Shift-click's new window open a page too.
fn new_page_for(disposition: WindowOpenDisposition) -> Option<bool> {
    use cef::sys::cef_window_open_disposition_t as Wod;
    match *disposition.as_ref() {
        Wod::CEF_WOD_NEW_FOREGROUND_TAB | Wod::CEF_WOD_NEW_POPUP | Wod::CEF_WOD_NEW_WINDOW => {
            Some(true)
        }
        Wod::CEF_WOD_NEW_BACKGROUND_TAB => Some(false),
        _ => None,
    }
}

impl PageContext {
    /// Tells the app the page asked for `url` in a new tab.
    fn open_tab(&self, url: Option<&CefString>, foreground: bool) {
        let url = url.map(ToString::to_string).unwrap_or_default();
        tracing::info!(page = %self.id, url, foreground, "link opens a new page");
        self.push(PageEvent::OpenTab {
            page: self.id,
            url,
            foreground,
        });
    }
}

wrap_request_handler! {
    pub(super) struct PageRequestHandler {
        ctx: PageContext,
    }

    impl RequestHandler {
        /// A Command-click or a middle click on a plain link. Cancelled
        /// here, and opened as a page beside this one.
        fn on_open_urlfrom_tab(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            target_url: Option<&CefString>,
            target_disposition: WindowOpenDisposition,
            _user_gesture: c_int,
        ) -> c_int {
            match new_page_for(target_disposition) {
                Some(foreground) => {
                    self.ctx.open_tab(target_url, foreground);
                    1
                }
                None => 0,
            }
        }

        fn on_render_process_terminated(
            &self,
            _browser: Option<&mut Browser>,
            status: TerminationStatus,
            error_code: c_int,
            _error_string: Option<&CefString>,
        ) {
            let reason = termination_reason(status);
            tracing::warn!(page = %self.ctx.id, reason, error_code, "renderer gone");
            self.ctx.push(PageEvent::Crashed {
                page: self.ctx.id,
                reason: reason.to_owned(),
            });
        }
    }
}

wrap_life_span_handler! {
    pub(super) struct PageLifeSpanHandler {
        ctx: PageContext,
    }

    impl LifeSpanHandler {
        fn on_before_popup(
            &self,
            _browser: Option<&mut Browser>,
            _frame: Option<&mut Frame>,
            _popup_id: c_int,
            target_url: Option<&CefString>,
            _target_frame_name: Option<&CefString>,
            target_disposition: WindowOpenDisposition,
            _user_gesture: c_int,
            _popup_features: Option<&PopupFeatures>,
            _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>,
            _no_javascript_access: Option<&mut c_int>,
        ) -> c_int {
            // `window.open` and target=_blank would otherwise open a native
            // window outside the canvas. Always cancelled; the app opens a
            // page on the canvas instead.
            let foreground = new_page_for(target_disposition).unwrap_or(true);
            self.ctx.open_tab(target_url, foreground);
            1
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            self.ctx.browser_closed();
        }
    }
}
