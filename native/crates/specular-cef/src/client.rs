//! CEF handler objects: the app (command-line switches) and one client per
//! page (render, display, load, request, life-span and keyboard handlers).
//!
//! The `wrap_*!` macros generate the ref-counted C vtables; each handler
//! holds a [`PageContext`] clone and only translates the callback into page
//! state or a [`PageEvent`].
#![expect(
    clippy::transmute_ptr_to_ptr,
    reason = "cef's wrap_* macros transmute the ref-count base in their expansion"
)]

mod display;
mod load;
mod render;

use self::display::PageDisplayHandler;
use self::load::{PageLifeSpanHandler, PageLoadHandler, PageRequestHandler};
use self::render::PageRenderHandler;

use std::os::raw::c_int;

use cef::{
    App, Browser, BrowserProcessHandler, CefString, Client, CommandLine, DisplayHandler, ImplApp,
    ImplBrowserProcessHandler, ImplClient, ImplCommandLine, ImplKeyboardHandler, KeyEvent,
    KeyboardHandler, LifeSpanHandler, LoadHandler, RenderHandler, RequestHandler, WrapApp,
    WrapBrowserProcessHandler, WrapClient, WrapKeyboardHandler, wrap_app,
    wrap_browser_process_handler, wrap_client, wrap_keyboard_handler,
};
// The `wrap_*!` expansions call `add_ref` from this trait unqualified.
use cef::rc::Rc as _;

use crate::config::Switch;
use crate::page::PageContext;

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

wrap_keyboard_handler! {
    struct PageKeyboardHandler;

    impl KeyboardHandler {
        /// A key the page did not consume, which is every plain key typed
        /// into a text field, ends here. Left alone, CEF offers it to the
        /// application's menu bar as a key equivalent: `e` opens Emoji &
        /// Symbols, and a letter that is a greyed tool's key beeps. The app
        /// chose to give the page this key, so nothing else is owed it.
        fn on_key_event(
            &self,
            _browser: Option<&mut Browser>,
            _event: Option<&KeyEvent>,
            _os_event: *mut u8,
        ) -> c_int {
            1
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
        keyboard: KeyboardHandler,
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

        fn keyboard_handler(&self) -> Option<KeyboardHandler> {
            Some(self.keyboard.clone())
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
        PageKeyboardHandler::new(),
    )
}
