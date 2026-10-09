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
use std::time::{Duration, Instant};

use futures::channel::mpsc;
use specular_app::{Bench, Runtime, ShellWindow as _};
use specular_interact::{
    Appearance, ChatModel, ControlsModel, Event, Menu, OnboardingModel, SidebarModel, ToolbarModel,
    ViewStrip, chat, dock, menus, onboarding, sidebar, toolbar, view_strip,
};

use crate::surface::{CanvasSurface, WindowAsks};

/// The refresh interval taken for a display that does not say its own.
const DEFAULT_REFRESH: Duration = Duration::from_nanos(16_666_667);

/// How early a display link's tick may come.
const TICK_SLACK: Duration = Duration::from_millis(2);

/// The least time between two readings of the models while the app keeps
/// changing. Building them walks the whole document, every frame of a pan
/// if nothing held it back, and GPUI lays its whole tree out again when one
/// changed, as the toolbar's zoom readout does every frame of a zoom. Both
/// happen on the thread the canvas draws from.
const MODELS_INTERVAL: Duration = Duration::from_millis(100);

/// Holds the reading of the models to one a [`MODELS_INTERVAL`]: the first
/// at once, and whatever changed meanwhile in one reading when the interval
/// is over, so GPUI always ends up showing the app as it is.
#[derive(Debug, Default)]
struct ModelGate {
    opened_at: Option<Instant>,
    owed: bool,
}

impl ModelGate {
    /// The app may have changed.
    fn ask(&mut self) {
        self.owed = true;
    }

    /// Whether to read the models now. Asked every turn.
    fn open(&mut self, now: Instant) -> bool {
        let rested = (self.opened_at).is_none_or(|at| now.duration_since(at) >= MODELS_INTERVAL);
        if !self.owed || !rested {
            return false;
        }
        self.owed = false;
        self.opened_at = Some(now);
        true
    }
}

/// How long after the last owed frame the display link is paused. A page
/// painting a few times a second keeps it running; a canvas nobody touches
/// stops being woken every refresh.
const REST_BEFORE_PAUSE: Duration = Duration::from_secs(1);

/// Whether the display link should be running: it is paused once no frame
/// has been owed for [`REST_BEFORE_PAUSE`].
#[derive(Debug)]
struct LinkRest {
    owed_at: Instant,
    /// Whether the link is paused now. The shell's timer then turns the
    /// canvas.
    paused: bool,
}

impl LinkRest {
    /// A frame is owed, or the app changed and one may be.
    fn owed(&mut self, now: Instant) {
        self.owed_at = now;
    }

    fn is_over(&self, now: Instant) -> bool {
        now.duration_since(self.owed_at) >= REST_BEFORE_PAUSE
    }
}

/// What GPUI draws from, as `update` last left it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Models {
    /// The tabs of the tab row: the canvas, then its pages and Documents.
    pub(crate) strip: ViewStrip,
    /// The tool buttons and the zoom readout.
    pub(crate) toolbar: ToolbarModel,
    /// What the dock shows: the controls of the tool in hand or of the
    /// selection.
    pub(crate) dock: Option<ControlsModel>,
    /// The left sidebar.
    pub(crate) sidebar: SidebarModel,
    /// The right panel: the canvas's agent threads and the composer.
    pub(crate) chat: ChatModel,
    /// The menu bar's menus that come from the app.
    pub(crate) menus: Vec<Menu>,
    /// The first-run view, while no space is open.
    pub(crate) onboarding: Option<OnboardingModel>,
    /// Light or dark: what the Kit's theme has to be.
    pub(crate) appearance: Appearance,
}

impl Models {
    fn of(runtime: &Runtime<CanvasSurface>) -> Self {
        let app = runtime.app();
        Self {
            strip: view_strip(app),
            toolbar: toolbar(app),
            dock: dock(app),
            sidebar: sidebar(app),
            chat: chat(app),
            menus: menus(app),
            onboarding: onboarding(app),
            appearance: app.appearance(),
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
    space_dialog: Option<bool>,
}

/// How long frames may follow one another without waiting for the display
/// link, from the last frame the link or a timer started.
const CATCH_UP_SPAN: Duration = Duration::from_millis(100);

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
        space_dialog: None,
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

/// The display link fired: one turn, and a frame if one is owed.
pub(crate) fn on_display_link() {
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

    /// The app may have changed: brings the models in step and wakes GPUI
    /// if one changed, now or when the gate next opens.
    pub(crate) fn refresh_models(&mut self) {
        let now = Instant::now();
        self.rest.owed(now);
        self.pause_link(false);
        self.stale = true;
        self.gate.ask();
        self.sync_models(now);
    }

    /// Pauses the display link once the canvas has rested a while, and
    /// starts it again when a frame is owed.
    fn pace_link(&mut self, now: Instant, wanted: bool) {
        if wanted {
            self.rest.owed(now);
        }
        self.pause_link(self.rest.is_over(now));
    }

    fn pause_link(&mut self, paused: bool) {
        if paused == self.rest.paused {
            return;
        }
        self.rest.paused = paused;
        if let Some(surface) = self.runtime.window() {
            surface.native().set_link_paused(paused);
        }
    }

    /// How long the shell's timer waits before it turns the canvas: the
    /// time to the next turn while the link is paused, and `otherwise`
    /// while the link runs and the timer only covers for a window the
    /// system gives no refreshes.
    pub(crate) fn timer_wait(&self, otherwise: Duration) -> Duration {
        match self.rest_until {
            Some(until) if self.rest.paused => {
                (until.saturating_duration_since(Instant::now())).min(otherwise)
            }
            _ => otherwise,
        }
    }

    /// Reads the models if the gate lets it and wakes GPUI when one changed
    /// or an effect asked something of the window. Called every turn.
    fn sync_models(&mut self, now: Instant) {
        if self.gate.open(now) {
            self.read_models();
            if std::mem::take(&mut self.unseen) {
                self.wake_now();
            }
        }
        let asked = self.runtime.take_space_dialog();
        if asked.is_some() {
            self.space_dialog = asked;
        }
        if asked.is_some() || self.asks.changed.get() {
            self.wake_now();
        }
    }

    /// The folder dialog the app asked for, once.
    pub(crate) fn take_space_dialog(&mut self) -> Option<bool> {
        self.space_dialog.take()
    }

    /// Builds the models again if the app may have changed since they were.
    fn read_models(&mut self) {
        if !std::mem::take(&mut self.stale) {
            return;
        }
        let models = Models::of(&self.runtime);
        if models != self.models {
            self.models = models;
            self.unseen = true;
        }
    }

    fn wake_now(&mut self) {
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
        if wanted && let Some(sample) = self.runtime.draw() {
            if let Some(bench) = self.bench.as_mut() {
                bench.presented(&self.runtime, &sample);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gesture_has_the_models_read_at_once_then_ten_times_a_second_and_once_when_it_ends() {
        let start = Instant::now();
        let frame = Duration::from_nanos(8_333_333);
        let mut gate = ModelGate::default();
        let mut readings = Vec::new();
        // A second of a pan changes the app every frame; then nothing does.
        for step in 0..240_u32 {
            let now = start + frame * step;
            if step < 120 {
                gate.ask();
            }
            if gate.open(now) {
                readings.push(step);
            }
        }
        assert_eq!(readings[0], 0);
        assert!((10..=11).contains(&readings.len()), "{readings:?}");
        // The last change, at frame 119, is shown: by a reading after it.
        assert!(
            readings.last().is_some_and(|&last| last >= 119),
            "{readings:?}"
        );
    }

    #[test]
    fn the_display_link_is_paused_a_second_after_the_last_owed_frame_and_no_sooner() {
        let start = Instant::now();
        let mut rest = LinkRest {
            owed_at: start,
            paused: false,
        };
        // A page painting twice a second keeps the link running.
        for half_seconds in 1..=4_u32 {
            let now = start + Duration::from_millis(500) * half_seconds;
            assert!(!rest.is_over(now));
            rest.owed(now);
        }
        let last = start + Duration::from_secs(2);
        assert!(!rest.is_over(last + Duration::from_millis(999)));
        assert!(rest.is_over(last + REST_BEFORE_PAUSE));
        rest.owed(last + Duration::from_secs(5));
        assert!(!rest.is_over(last + Duration::from_secs(5)));
    }
}
