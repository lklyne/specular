//! The app and its canvas, owned beside GPUI's entities.
//!
//! The [`Runtime`] is held in a thread-local and not inside a GPUI entity,
//! because the canvas draws from its own display link, outside any GPUI
//! update. Everything is on the main thread. GPUI views send events through
//! [`dispatch`] and read the pure models from [`models`]; the frame tells
//! GPUI when a model changed, so `update` stays the only thing that changes
//! what either renderer shows.

mod models;
mod pacing;

pub(crate) use self::models::Models;
use self::pacing::{LinkRest, ModelGate};

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use futures::channel::mpsc;
use specular_interact::panel::ContextMenu;
use specular_interact::{ControlsModel, Event, context_menu};
use specular_shell::{Bench, Runtime, ShellWindow as _};

use crate::surface::{CanvasSurface, WindowAsks};

/// The refresh interval taken for a display that does not say its own.
const DEFAULT_REFRESH: Duration = Duration::from_nanos(16_666_667);

/// How early a display link's tick may come.
const TICK_SLACK: Duration = Duration::from_millis(2);

/// The runtime with what ties it to GPUI.
pub(crate) struct Canvas {
    /// The app and its effect runners.
    pub(crate) runtime: Runtime<CanvasSurface>,
    /// What effects asked of the window.
    pub(crate) asks: Rc<WindowAsks>,
    models: Models,
    /// Whether the app may have changed since the models were read.
    stale: bool,
    /// Whether the models changed since GPUI was last told.
    unseen: bool,
    /// Paces the reading of the models.
    gate: ModelGate,
    /// Tells GPUI to draw again.
    wake: mpsc::Sender<()>,
    /// When the canvas last ran a frame.
    last_frame: Instant,
    /// Asks the shell for another frame at once, without waiting for the
    /// display link.
    again: mpsc::Sender<()>,
    /// The display's refresh interval.
    refresh: Duration,
    /// When the display link is next expected to fire.
    next_tick: Instant,
    /// When the last frame that was not a catch-up frame started.
    chain_start: Instant,
    /// The benchmark, in a `--bench` run.
    bench: Option<Bench>,
    /// While nothing is owed, when the next full turn is due. Until then a
    /// turn only looks at the pages.
    rest_until: Option<Instant>,
    /// When to pause the display link, and whether it is.
    rest: LinkRest,
    /// Set once the page backend has shut down after an exit.
    finished: bool,
    /// A folder dialog the app asked for, until GPUI shows it: whether it
    /// is worded for making a space.
    folder_dialog: Option<specular_shell::FolderAsk>,
    /// The context menu GPUI was last told of. It skips the models' gate,
    /// so a right press shows its menu on the next frame.
    menu: Option<ContextMenu>,
}

/// How long frames may follow one another without waiting for the display
/// link, from the last frame the link or a timer started.
const CATCH_UP_SPAN: Duration = Duration::from_millis(100);

/// Where the canvas lives. A quit takes it out and shuts it down in order
/// ([`uninstall`]). A process ended by a signal still has it here when the
/// thread's locals are torn down, where closing the pages cannot work: the
/// locals that shutdown needs, the log's among them, may be gone first. So
/// what is left then is let go of without being dropped.
struct Slot(RefCell<Option<Canvas>>);

impl Drop for Slot {
    fn drop(&mut self) {
        if let Some(canvas) = self.0.get_mut().take() {
            let _ = std::mem::ManuallyDrop::new(canvas);
        }
    }
}

thread_local! {
    static CANVAS: Slot = const { Slot(RefCell::new(None)) };
}

/// Runs `with` on the canvas, unless there is none or it is in use: a frame
/// asked for while an event is being handled is skipped.
pub(crate) fn with<R>(with: impl FnOnce(&mut Canvas) -> R) -> Option<R> {
    CANVAS.with(|cell| {
        let mut slot = cell.0.try_borrow_mut().ok()?;
        slot.as_mut().map(with)
    })
}

/// Makes `runtime` the canvas. GPUI is told to draw again through `wake`,
/// the shell runs [`Canvas::frame`] once more for each word on `again`, and
/// `bench` is stepped a frame.
pub(crate) fn install(
    runtime: Runtime<CanvasSurface>,
    asks: Rc<WindowAsks>,
    (wake, again): (mpsc::Sender<()>, mpsc::Sender<()>),
    refresh: Option<Duration>,
    bench: Option<Bench>,
) {
    let models = Models::of(&runtime);
    let canvas = Canvas {
        runtime,
        asks,
        models,
        stale: false,
        unseen: false,
        gate: ModelGate::default(),
        wake,
        last_frame: Instant::now(),
        again,
        refresh: refresh.unwrap_or(DEFAULT_REFRESH),
        next_tick: Instant::now(),
        chain_start: Instant::now(),
        bench,
        rest_until: None,
        rest: LinkRest {
            owed_at: Instant::now(),
            paused: false,
        },
        finished: false,
        folder_dialog: None,
        menu: None,
    };
    CANVAS.with(|cell| *cell.0.borrow_mut() = Some(canvas));
}

/// Takes the canvas out, at the end of the run.
pub(crate) fn uninstall() -> Option<Canvas> {
    CANVAS.with(|cell| cell.0.try_borrow_mut().ok()?.take())
}

/// Sends `event` through `update` and runs its effects.
pub(crate) fn dispatch(event: Event) {
    with(|canvas| canvas.dispatch(event));
}

/// The models as the app is now.
pub(crate) fn models() -> Option<Models> {
    with(|canvas| {
        canvas.read_models();
        if canvas.unseen {
            // Read early by a handler, not by GPUI's render: it is told
            // when the gate next opens.
            canvas.gate.ask();
        }
        canvas.models.clone()
    })
}

/// The context menu that is open, with its items as the app has them now.
pub(crate) fn open_menu() -> Option<(ContextMenu, ControlsModel)> {
    with(|canvas| {
        let app = canvas.runtime.app();
        let open = app.session().panel.menu.clone()?;
        let model = context_menu(app, &open.target)?;
        Some((open, model))
    })
    .flatten()
}

/// The display link fired: one turn, and a frame if one is owed.
pub(crate) fn on_display_link() {
    specular_core::ledger::record(specular_core::ledger::Entry::Tick);
    with(|canvas| {
        canvas.next_tick = Instant::now() + canvas.refresh;
        canvas.frame();
    });
}

impl Canvas {
    /// The frame a late one asked for, without waiting for the display link.
    pub(crate) fn catch_up(&mut self) {
        self.run_frame();
    }

    /// Sends `event` through `update`, then brings the models in step.
    pub(crate) fn dispatch(&mut self, event: Event) {
        if matches!(
            event,
            Event::Pointer(_) | Event::Wheel(_) | Event::Pinch { .. } | Event::Key(_)
        ) {
            // Frames keep coming for a moment after the user did something.
            self.runtime.input();
        }
        self.rest_until = None;
        self.runtime.dispatch(event);
        let paused = self.rest.paused;
        self.refresh_models();
        if paused {
            // The link's next tick may be a while off on a display that
            // slowed down while nothing was presented.
            self.frame();
        }
    }

    /// The folder dialog the app asked for, once.
    pub(crate) fn take_folder_dialog(&mut self) -> Option<specular_shell::FolderAsk> {
        self.folder_dialog.take()
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
    /// frame on screen if any of that, or an event since the last turn,
    /// changed what a frame shows. An idle canvas draws nothing.
    pub(crate) fn frame(&mut self) {
        self.chain_start = Instant::now();
        self.run_frame();
    }

    /// [`Self::frame`], which asks for a catch-up frame if it runs late.
    fn run_frame(&mut self) {
        let now = Instant::now();
        self.last_frame = now;
        if self.runtime.is_closing() {
            if !self.finished && self.runtime.poll_shutdown() {
                self.finished = true;
                self.wake_now();
            }
            return;
        }
        if self.rest_until.is_some_and(|until| now < until) {
            self.runtime.take_pages();
            if !self.runtime.frame_wanted() {
                self.sync_models(now);
                self.pace_link(now, false);
                return;
            }
        }
        self.rest_until = None;
        let traced_at = specular_core::ledger::now_us();
        let configured = self.runtime.window().map(CanvasSurface::pixel_size);
        let rescaled = self.runtime.window_mut().and_then(CanvasSurface::sync);
        if let Some(scale) = rescaled {
            self.runtime.on_scale_factor_changed(f64::from(scale));
        }
        self.runtime.turn();
        let mut bench_turn = None;
        match self.bench.as_mut() {
            Some(bench) => {
                bench_turn = bench.step(&mut self.runtime, now);
                if self.runtime.is_closing() {
                    return;
                }
            }
            // A benchmark window keeps the title it opened with.
            None => self.runtime.refresh_title(),
        }
        let wanted = self.runtime.frame_wanted();
        let drawn = if wanted { self.runtime.draw() } else { None };
        if specular_core::ledger::enabled() {
            match self.runtime.window().map(CanvasSurface::pixel_size) {
                Some(size) if drawn.is_some() => {
                    specular_core::ledger::frame(traced_at, size, configured != Some(size));
                }
                _ => specular_core::ledger::frame_abandoned(),
            }
        }
        if let Some(sample) = drawn {
            self.presented(&sample);
        }
        // Whatever changes the app owes a frame, so a turn that owed none
        // has nothing new for GPUI either.
        let started = Instant::now();
        if wanted {
            self.stale = true;
            self.gate.ask();
        }
        self.sync_models(now);
        if let Some(bench) = self.bench.as_mut() {
            bench.worked(started.elapsed());
        }
        self.pace_link(now, wanted);
        if !wanted {
            let turn = self.runtime.next_turn();
            // The link's ticks are not exact, and a step due at the next
            // one must not be put off by a tick that comes a moment early.
            let bench_turn = bench_turn.map(|at| at.checked_sub(TICK_SLACK).unwrap_or(at));
            self.rest_until = Some(bench_turn.map_or(turn, |bench| turn.min(bench)));
        }
    }

    /// A frame reached the screen: the benchmark hears of it, and a frame
    /// that ran past the link's next tick asks for the next at once.
    fn presented(&mut self, sample: &specular_compositor::FrameSample) {
        if let Some(bench) = self.bench.as_mut() {
            bench.presented(&self.runtime, sample);
        }
        let end = Instant::now();
        if end >= self.next_tick {
            // This frame ran past the link's next tick, which is lost.
            // The next frame starts at once and not a refresh later; the
            // layer's drawables pace it, as they pace a loop that blocks
            // on the display.
            while self.next_tick <= end {
                self.next_tick += self.refresh;
            }
            // For a tenth of a second and no longer: without the limit
            // the main thread never gets back to its run loop for as
            // long as frames overrun, and no input arrives.
            if end.duration_since(self.chain_start) < CATCH_UP_SPAN {
                let _ = self.again.try_send(());
            }
        }
    }

    /// The canvas slot's place in the window, from GPUI's layout.
    pub(crate) fn set_slot(&mut self, origin: glam::Vec2, size: glam::Vec2) {
        let resized =
            (self.runtime.window_mut()).is_some_and(|surface| surface.set_slot(origin, size));
        if resized {
            specular_core::ledger::record(specular_core::ledger::Entry::Slot(size.x, size.y));
            let viewport = (self.runtime.window()).map(CanvasSurface::logical_viewport);
            if let Some(viewport) = viewport {
                self.dispatch(Event::ViewportResized(viewport));
            }
        }
    }
}
