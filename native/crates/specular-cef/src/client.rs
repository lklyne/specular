//! CEF handler objects: the app (command-line switches) and one client per
//! page (render, display, load, request and life-span handlers).
//!
//! The `wrap_*!` macros generate the ref-counted C vtables; each handler
//! holds a [`PageContext`] clone and only translates the callback into page
//! state or a [`PageEvent`].
#![expect(
    clippy::transmute_ptr_to_ptr,
    reason = "cef's wrap_* macros transmute the ref-count base in their expansion"
)]

use std::os::raw::c_int;

use cef::{
    AcceleratedPaintInfo, App, Browser, BrowserProcessHandler, BrowserSettings, CefString, Client,
    CommandLine, DictionaryValue, DisplayHandler, Frame, ImplApp, ImplBrowserProcessHandler,
    ImplClient, ImplCommandLine, ImplDisplayHandler, ImplFrame, ImplLifeSpanHandler,
    ImplLoadHandler, ImplRenderHandler, ImplRequestHandler, LifeSpanHandler, LoadHandler,
    PaintElementType, PopupFeatures, Range, Rect, RenderHandler, RequestHandler, ScreenInfo,
    TerminationStatus, WindowInfo, WindowOpenDisposition, WrapApp, WrapBrowserProcessHandler,
    WrapClient, WrapDisplayHandler, WrapLifeSpanHandler, WrapLoadHandler, WrapRenderHandler,
    WrapRequestHandler, wrap_app, wrap_browser_process_handler, wrap_client, wrap_display_handler,
    wrap_life_span_handler, wrap_load_handler, wrap_render_handler, wrap_request_handler,
};
// The `wrap_*!` expansions call `add_ref` from this trait unqualified.
use cef::rc::Rc as _;
use specular_core::PageEvent;

use crate::config::Switch;
use crate::coords::{rect_from_cef, union_rects};
use crate::cpu_frame::bgra_len;
use crate::page::PageContext;
use crate::paint;

wrap_app! {
    pub(crate) struct SpecularApp {
        switches: Vec<Switch>,
    }

    impl App {
        fn on_before_command_line_processing(
            &self,
            process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            let is_browser = process_type.is_none_or(|kind| kind.to_string().is_empty());
            let Some(command_line) = command_line else {
                return;
            };
            if !is_browser {
                return;
            }
            for switch in &self.switches {
                let name = CefString::from(switch.name);
                match switch.value {
                    Some(value) => command_line
                        .append_switch_with_value(Some(&name), Some(&CefString::from(value))),
                    None => command_line.append_switch(Some(&name)),
                }
            }
        }

        fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
            Some(PumpScheduler::new())
        }
    }
}

wrap_browser_process_handler! {
    struct PumpScheduler;

    impl BrowserProcessHandler {
        fn on_schedule_message_pump_work(&self, delay_ms: i64) {
            #[cfg(target_os = "macos")]
            crate::pump_timer::schedule(delay_ms);
            // Elsewhere the host pumps every turn of its own loop.
            #[cfg(not(target_os = "macos"))]
            let _ = delay_ms;
        }
    }
}

/// The view rect CEF asks for: the page's CSS viewport at the origin.
/// CEF requires a non-empty rect.
fn view_rect(ctx: &PageContext) -> Rect {
    let viewport = ctx.geometry().viewport;
    Rect {
        x: 0,
        y: 0,
        width: viewport.width.max(1) as i32,
        height: viewport.height.max(1) as i32,
    }
}

wrap_render_handler! {
    struct PageRenderHandler {
        ctx: PageContext,
    }

    impl RenderHandler {
        fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
            if let Some(rect) = rect {
                *rect = view_rect(&self.ctx);
            }
        }

        fn screen_info(
            &self,
            _browser: Option<&mut Browser>,
            screen_info: Option<&mut ScreenInfo>,
        ) -> c_int {
            let Some(info) = screen_info else {
                return 0;
            };
            // The "screen" is the view itself, so Chromium keeps popups
            // (<select> lists, date pickers) inside the page's texture.
            let rect = view_rect(&self.ctx);
            info.device_scale_factor = self.ctx.geometry().scale;
            info.rect = rect.clone();
            info.available_rect = rect;
            1
        }

        fn screen_point(
            &self,
            _browser: Option<&mut Browser>,
            view_x: c_int,
            view_y: c_int,
            screen_x: Option<&mut c_int>,
            screen_y: Option<&mut c_int>,
        ) -> c_int {
            // View and screen coincide (see `screen_info`).
            let (Some(screen_x), Some(screen_y)) = (screen_x, screen_y) else {
                return 0;
            };
            *screen_x = view_x;
            *screen_y = view_y;
            1
        }

        fn on_popup_show(&self, _browser: Option<&mut Browser>, show: c_int) {
            let visible = show != 0;
            if !visible {
                self.ctx.geometry().clear_popup();
            }
            self.ctx.push(PageEvent::PopupVisibility {
                page: self.ctx.id,
                visible,
            });
        }

        fn on_popup_size(&self, _browser: Option<&mut Browser>, rect: Option<&Rect>) {
            let Some(rect) = rect else {
                return;
            };
            let css = rect_from_cef(rect.x, rect.y, rect.width, rect.height);
            let placed = self.ctx.geometry().set_popup(css);
            self.ctx.push(PageEvent::PopupRect {
                page: self.ctx.id,
                rect: placed,
            });
        }

        fn on_paint(
            &self,
            _browser: Option<&mut Browser>,
            type_: PaintElementType,
            dirty_rects: Option<&[Rect]>,
            buffer: *const u8,
            width: c_int,
            height: c_int,
        ) {
            let Some(len) = bgra_len(width, height) else {
                return;
            };
            if buffer.is_null() {
                return;
            }
            // SAFETY: CEF documents `buffer` as `width * height * 4` bytes of
            // BGRA, valid for the duration of OnPaint; `len` is exactly that
            // size, and the slice does not outlive this callback (`on_paint`
            // copies it before returning).
            let bytes = unsafe { std::slice::from_raw_parts(buffer, len) };
            paint::on_paint(&self.ctx, type_, dirty_rects, bytes, width, height);
        }

        fn on_accelerated_paint(
            &self,
            _browser: Option<&mut Browser>,
            type_: PaintElementType,
            _dirty_rects: Option<&[Rect]>,
            info: Option<&AcceleratedPaintInfo>,
        ) {
            #[cfg(target_os = "macos")]
            paint::on_accelerated_paint(&self.ctx, type_, info);
            #[cfg(not(target_os = "macos"))]
            {
                // Shared textures are only requested on macOS
                // (`CefConfig::uses_shared_texture`).
                let _ = (type_, info);
                tracing::warn!(page = %self.ctx.id, "unexpected accelerated paint off macOS");
            }
        }

        fn on_scroll_offset_changed(&self, _browser: Option<&mut Browser>, x: f64, y: f64) {
            self.ctx.push(PageEvent::Scrolled {
                page: self.ctx.id,
                offset: glam::Vec2::new(x as f32, y as f32),
            });
        }

        fn on_ime_composition_range_changed(
            &self,
            _browser: Option<&mut Browser>,
            _selected_range: Option<&Range>,
            character_bounds: Option<&[Rect]>,
        ) {
            let bounds = union_rects(
                character_bounds
                    .unwrap_or_default()
                    .iter()
                    .map(|r| rect_from_cef(r.x, r.y, r.width, r.height)),
            );
            self.ctx.push(PageEvent::ImeCompositionBounds {
                page: self.ctx.id,
                bounds,
            });
        }
    }
}

wrap_load_handler! {
    struct PageLoadHandler {
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

        fn on_load_end(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            http_status_code: c_int,
        ) {
            if frame.is_some_and(|frame| frame.is_main() != 0) {
                self.ctx.push(PageEvent::Loaded {
                    page: self.ctx.id,
                    http_status: http_status_code,
                });
            }
        }
    }
}

wrap_display_handler! {
    struct PageDisplayHandler {
        ctx: PageContext,
    }

    impl DisplayHandler {
        fn on_address_change(
            &self,
            _browser: Option<&mut Browser>,
            frame: Option<&mut Frame>,
            url: Option<&CefString>,
        ) {
            // A subframe's address is not the page's.
            if frame.is_some_and(|frame| frame.is_main() != 0) {
                self.ctx.push(PageEvent::Url {
                    page: self.ctx.id,
                    url: url.map(ToString::to_string).unwrap_or_default(),
                });
            }
        }

        fn on_title_change(&self, _browser: Option<&mut Browser>, title: Option<&CefString>) {
            self.ctx.push(PageEvent::Title {
                page: self.ctx.id,
                title: title.map(ToString::to_string).unwrap_or_default(),
            });
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

wrap_request_handler! {
    struct PageRequestHandler {
        ctx: PageContext,
    }

    impl RequestHandler {
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
    struct PageLifeSpanHandler {
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
            _target_disposition: WindowOpenDisposition,
            _user_gesture: c_int,
            _popup_features: Option<&PopupFeatures>,
            _window_info: Option<&mut WindowInfo>,
            _client: Option<&mut Option<Client>>,
            _settings: Option<&mut BrowserSettings>,
            _extra_info: Option<&mut Option<DictionaryValue>>,
            _no_javascript_access: Option<&mut c_int>,
        ) -> c_int {
            // `window.open` / target=_blank would otherwise open a native
            // (non-OSR) window outside the canvas. Blocked: new pages on the
            // canvas are an app decision, not the page's.
            let url = target_url.map(ToString::to_string).unwrap_or_default();
            tracing::info!(page = %self.ctx.id, url, "blocked window.open popup");
            1
        }

        fn on_before_close(&self, _browser: Option<&mut Browser>) {
            self.ctx.browser_closed();
        }
    }
}

wrap_client! {
    struct PageClient {
        render: RenderHandler,
        display: DisplayHandler,
        load: LoadHandler,
        request: RequestHandler,
        life_span: LifeSpanHandler,
    }

    impl Client {
        fn render_handler(&self) -> Option<RenderHandler> {
            Some(self.render.clone())
        }

        fn display_handler(&self) -> Option<DisplayHandler> {
            Some(self.display.clone())
        }

        fn load_handler(&self) -> Option<LoadHandler> {
            Some(self.load.clone())
        }

        fn request_handler(&self) -> Option<RequestHandler> {
            Some(self.request.clone())
        }

        fn life_span_handler(&self) -> Option<LifeSpanHandler> {
            Some(self.life_span.clone())
        }
    }
}

/// The app object for `cef_initialize` in the browser process.
pub(crate) fn new_app(switches: Vec<Switch>) -> App {
    SpecularApp::new(switches)
}

/// A client whose handlers all report into `ctx`.
pub(crate) fn new_client(ctx: &PageContext) -> Client {
    PageClient::new(
        PageRenderHandler::new(ctx.clone()),
        PageDisplayHandler::new(ctx.clone()),
        PageLoadHandler::new(ctx.clone()),
        PageRequestHandler::new(ctx.clone()),
        PageLifeSpanHandler::new(ctx.clone()),
    )
}
