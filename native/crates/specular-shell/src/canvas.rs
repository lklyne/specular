//! The app and its canvas, owned beside GPUI's entities.
//!
//! The [`Runtime`] is held in a thread-local and not inside a GPUI entity,
//! because the canvas draws from its own display link, outside any GPUI
//! update. Everything is on the main thread. GPUI views send events through
//! [`dispatch`] and read the pure models from [`models`]; the frame tells
//! GPUI when a model changed, so `update` stays the only thing that changes
//! what either renderer shows.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use futures::channel::mpsc;
use specular_app::{Runtime, ShellWindow as _};
use specular_interact::{
    ChatModel, Event, Menu, PopupAnchor, PopupModel, SidebarModel, ToolbarModel, chat, menus,
    popup_for, sidebar, toolbar,
};

use crate::surface::{CanvasSurface, WindowAsks};

/// What GPUI draws from, as `update` last left it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Models {
    /// The tool buttons and the zoom readout.
    pub(crate) toolbar: ToolbarModel,
    /// The popup hung from the toolbar, when the tool in hand has one. A
    /// popup beside a canvas item is drawn in the canvas's own pass.
    pub(crate) popup: Option<PopupModel>,
    /// The left sidebar.
    pub(crate) sidebar: SidebarModel,
    /// The right panel: the canvas's agent threads and the composer.
    pub(crate) chat: ChatModel,
    /// The menu bar's menus that come from the app.
    pub(crate) menus: Vec<Menu>,
}

impl Models {
    fn of(runtime: &Runtime<CanvasSurface>) -> Self {
        let app = runtime.app();
        Self {
            toolbar: toolbar(app),
            popup: popup_for(app)
                .filter(|popup| matches!(popup.anchor, PopupAnchor::Toolbar { .. })),
            sidebar: sidebar(app),
            chat: chat(app),
            menus: menus(app),
        }
    }
}

/// The runtime with what ties it to GPUI.
pub(crate) struct Canvas {
    /// The app and its effect runners.
    pub(crate) runtime: Runtime<CanvasSurface>,
    /// What effects asked of the window.
    pub(crate) asks: Rc<WindowAsks>,
    models: Models,
    /// Tells GPUI to draw again.
    wake: mpsc::Sender<()>,
    /// When the canvas last ran a frame.
    last_frame: Instant,
    /// Set once the page backend has shut down after an exit.
    finished: bool,
}

thread_local! {
    static CANVAS: RefCell<Option<Canvas>> = const { RefCell::new(None) };
}

/// Runs `with` on the canvas, unless there is none or it is in use: a frame
/// asked for while an event is being handled is skipped.
pub(crate) fn with<R>(with: impl FnOnce(&mut Canvas) -> R) -> Option<R> {
    CANVAS.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        slot.as_mut().map(with)
    })
}

/// Makes `runtime` the canvas. GPUI is told to draw again through `wake`.
pub(crate) fn install(
    runtime: Runtime<CanvasSurface>,
    asks: Rc<WindowAsks>,
    wake: mpsc::Sender<()>,
) {
    let models = Models::of(&runtime);
    let canvas = Canvas {
        runtime,
        asks,
        models,
        wake,
        last_frame: Instant::now(),
        finished: false,
    };
    CANVAS.with(|cell| *cell.borrow_mut() = Some(canvas));
}

/// Takes the canvas out, at the end of the run.
pub(crate) fn uninstall() -> Option<Canvas> {
    CANVAS.with(|cell| cell.try_borrow_mut().ok()?.take())
}

/// Sends `event` through `update` and runs its effects.
pub(crate) fn dispatch(event: Event) {
    with(|canvas| canvas.dispatch(event));
}

/// The models as they are now.
pub(crate) fn models() -> Option<Models> {
    with(|canvas| canvas.models.clone())
}

/// The display link fired: one turn and one frame.
pub(crate) fn on_display_link() {
    with(Canvas::frame);
}

impl Canvas {
    /// Sends `event` through `update`, then brings the models in step.
    pub(crate) fn dispatch(&mut self, event: Event) {
        self.runtime.dispatch(event);
        self.refresh_models();
    }

    /// Recomputes the models and wakes GPUI when one changed or an effect
    /// asked something of the window.
    pub(crate) fn refresh_models(&mut self) {
        let models = Models::of(&self.runtime);
        let changed = models != self.models;
        if changed {
            self.models = models;
        }
        if changed || self.asks.changed.get() {
            self.wake_gpui();
        }
    }

    fn wake_gpui(&mut self) {
        // A full channel already has a wake waiting.
        let _ = self.wake.try_send(());
    }

    /// Whether the run is over: exit was asked for and the pages are gone.
    pub(crate) fn is_finished(&self) -> bool {
        self.finished
    }

    /// When the canvas last ran a frame.
    pub(crate) fn last_frame(&self) -> Instant {
        self.last_frame
    }

    /// One loop turn: the clock, the files, what the pages reported, and a
    /// frame on screen.
    pub(crate) fn frame(&mut self) {
        self.last_frame = Instant::now();
        if self.runtime.is_closing() {
            if !self.finished && self.runtime.poll_shutdown() {
                self.finished = true;
                self.wake_gpui();
            }
            return;
        }
        let rescaled = self.runtime.window_mut().and_then(CanvasSurface::sync);
        if let Some(scale) = rescaled {
            self.runtime.on_scale_factor_changed(f64::from(scale));
        }
        self.runtime.turn();
        self.runtime.refresh_title();
        self.runtime.draw();
        self.refresh_models();
    }

    /// The canvas slot's place in the window, from GPUI's layout.
    pub(crate) fn set_slot(&mut self, origin: glam::Vec2, size: glam::Vec2) {
        let resized =
            (self.runtime.window_mut()).is_some_and(|surface| surface.set_slot(origin, size));
        if resized {
            let viewport = (self.runtime.window()).map(CanvasSurface::logical_viewport);
            if let Some(viewport) = viewport {
                self.dispatch(Event::ViewportResized(viewport));
            }
        }
    }
}
