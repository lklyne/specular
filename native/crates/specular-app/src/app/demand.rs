//! Whether there is anything new to draw.
//!
//! A frame is a function of the app, the page textures and the images, so
//! one is owed only when one of those changed. Everything that changes them
//! says so here, and a loop turn with nothing owed draws nothing: an idle
//! canvas costs no frames at all.

use std::time::{Duration, Instant};

use specular_core::Camera;
use specular_interact::{App, Effect, Event, Gesture, update};

/// How long the loop sleeps when nothing is owed: the slowest anything the
/// shell polls (decoded images, read notes, files edited outside) is seen.
pub(crate) const IDLE_TURN: Duration = Duration::from_millis(50);
/// The sleep while text is being edited, so the caret blinks on time.
const EDITING_TURN: Duration = Duration::from_millis(16);
/// How long frames keep coming after the last input. A display shown no
/// frames lowers its refresh rate within a few of them and takes several
/// more to raise it, so a gesture that pauses for a moment would resume at
/// half rate. Holding the rate through the pause costs a few frames of a
/// scene that is already built.
const INPUT_TAIL: Duration = Duration::from_millis(250);
/// How long to leave a window that gave no frame (covered, minimised)
/// before trying again.
const RETRY: Duration = Duration::from_millis(100);

/// The frames owed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameDemand {
    owed: bool,
    /// Set after a frame the window refused; nothing is drawn before it.
    not_before: Option<Instant>,
    /// Set by input until the next turn stamps it with the time.
    input: bool,
    /// Frames are drawn until then whether or not anything changed.
    tail_until: Option<Instant>,
    drawn: u64,
    skipped: u64,
}

impl Default for FrameDemand {
    fn default() -> Self {
        // The first frame is owed to an empty window.
        Self {
            owed: true,
            not_before: None,
            input: false,
            tail_until: None,
            drawn: 0,
            skipped: 0,
        }
    }
}

impl FrameDemand {
    /// Something a frame shows has changed.
    pub(crate) fn changed(&mut self) {
        self.owed = true;
    }

    /// The user did something: a pointer, wheel or key event, or a step of
    /// a scripted gesture. Frames then keep coming for [`INPUT_TAIL`].
    pub(crate) fn input(&mut self) {
        self.owed = true;
        self.input = true;
    }

    /// Whether to draw now. A turn that is asked and owes nothing is
    /// counted as skipped.
    pub(crate) fn wanted(&mut self, now: Instant) -> bool {
        if std::mem::take(&mut self.input) {
            self.tail_until = Some(now + INPUT_TAIL);
        }
        if self.tail_until.is_some_and(|until| now >= until) {
            self.tail_until = None;
        }
        let held = self.not_before.is_some_and(|until| now < until);
        if (self.owed || self.tail_until.is_some()) && !held {
            return true;
        }
        if !self.owed {
            self.skipped += 1;
        }
        false
    }

    /// A frame was drawn. `settling` asks for one more: the frame held
    /// something back (text at a zoom's old raster size) that the next
    /// puts right.
    pub(crate) fn drawn(&mut self, settling: bool) {
        self.owed = settling;
        self.not_before = None;
        self.drawn += 1;
    }

    /// The window gave no frame to draw into. Still owed, but not at once.
    pub(crate) fn refused(&mut self, now: Instant) {
        self.not_before = Some(now + RETRY);
    }

    /// When the loop should wake if no event comes first.
    pub(crate) fn next_turn(&self, app: &App, now: Instant) -> Instant {
        let turn = if app.session().editing.is_some() {
            EDITING_TURN
        } else {
            IDLE_TURN
        };
        let mut wake = now + turn;
        // A gif on screen is the one thing the clock alone redraws, and it
        // is woken for at its next frame and not at every refresh.
        if let Some(ms) = app.next_frame_in_ms() {
            wake = wake.min(now + Duration::from_millis(ms));
        }
        match self.not_before {
            Some(until) if self.owed => wake.min(until),
            _ => wake,
        }
    }

    /// Frames drawn and turns skipped so far.
    pub(crate) fn counts(&self) -> (u64, u64) {
        (self.drawn, self.skipped)
    }
}

/// What the clock alone can change on screen.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pulse {
    caret_shown: bool,
    camera: Camera,
    animation_frames: u64,
}

impl Pulse {
    fn of(app: &App) -> Self {
        Self {
            caret_shown: app.caret_visible(),
            camera: app.session().camera,
            animation_frames: app.animation_epoch(),
        }
    }
}

/// Sends the clock to the app. Returns its effects and whether the frame on
/// screen is now stale: the caret blinked, a held text selection scrolled,
/// or the tick did something.
pub(crate) fn tick(app: &mut App, unix_ms: u64) -> (Vec<Effect>, bool) {
    let before = Pulse::of(app);
    let effects = update(app, Event::Tick { unix_ms });
    // A selection dragged past the edge scrolls on the clock, the canvas or
    // a Document's own rows.
    let scrolling = matches!(app.session().gesture, Some(Gesture::TextSelect(_)));
    let stale = scrolling || !effects.is_empty() || Pulse::of(app) != before;
    (effects, stale)
}

#[cfg(test)]
mod tests {
    use specular_doc::Rect;
    use specular_interact::ImageNotice;
    use specular_testkit::{TestApp, file, sticky};

    use super::*;

    const START_MS: u64 = 1_700_000_000_000;

    /// Runs the clock for `seconds` at one tick a 120 Hz frame, as a loop
    /// that woke every refresh would, and returns the frames it would draw.
    fn frames_drawn(app: &mut App, from_ms: u64, seconds: u64) -> u64 {
        let mut demand = FrameDemand::default();
        let mut now = Instant::now();
        // The frame owed to the empty window.
        assert!(demand.wanted(now));
        demand.drawn(false);
        for step in 0..seconds * 120 {
            let (_, stale) = tick(app, from_ms + step * 1_000 / 120);
            if stale {
                demand.changed();
            }
            now += Duration::from_nanos(8_333_333);
            if demand.wanted(now) {
                demand.drawn(false);
            }
        }
        demand.counts().0 - 1
    }

    fn one_sticky() -> TestApp {
        TestApp::with_entities([sticky("s1", Rect::new(100.0, 100.0, 200.0, 200.0), "note")])
    }

    #[test]
    fn thirty_idle_seconds_draw_no_frame() {
        let mut app = one_sticky();
        app.tick(START_MS).select(&["s1"]);
        let mut app = app.app().clone();
        assert_eq!(frames_drawn(&mut app, START_MS, 30), 0);
    }

    /// A gif of three 30 ms frames at the document's one file entity.
    fn gif_at(x: f64) -> TestApp {
        let rect = Rect::new(x, 100.0, 200.0, 200.0);
        let mut app = TestApp::with_entities([file("g", rect)]);
        app.viewport((1000.0, 800.0)).tick(START_MS);
        let image = app.app().image("g.png").map(|image| image.key);
        let delays_ms: std::sync::Arc<[u32]> = std::sync::Arc::from([30, 30, 30]);
        let notice = ImageNotice::Animated {
            width: 8,
            height: 8,
            delays_ms,
        };
        if let Some(image) = image {
            app.send(Event::Image { image, notice });
        }
        app
    }

    #[test]
    fn a_gif_on_screen_draws_a_frame_at_each_of_its_delays_and_one_off_screen_draws_none() {
        // Two seconds at 120 Hz: about 67 frame changes, not 240 refreshes.
        let mut app = gif_at(100.0).app().clone();
        let drawn = frames_drawn(&mut app, START_MS, 2);
        assert!((60..=72).contains(&drawn), "{drawn} frames");
        let mut off_screen = gif_at(5_000.0).app().clone();
        assert_eq!(frames_drawn(&mut off_screen, START_MS, 2), 0);
        // And the loop sleeps until the next frame, not the idle turn.
        let demand = FrameDemand::default();
        let now = Instant::now();
        assert!(demand.next_turn(&app, now) <= now + Duration::from_millis(30));
        assert_eq!(demand.next_turn(&off_screen, now), now + IDLE_TURN);
    }

    #[test]
    fn an_open_caret_draws_two_frames_a_second_and_no_more() {
        let mut app = one_sticky();
        app.tick(START_MS).double_click((150.0, 150.0));
        assert!(app.session().editing.is_some());
        let mut app = app.app().clone();
        let drawn = frames_drawn(&mut app, START_MS, 30);
        assert!((58..=61).contains(&drawn), "{drawn} frames");
    }

    #[test]
    fn a_change_is_drawn_once() {
        let mut demand = FrameDemand::default();
        let now = Instant::now();
        demand.drawn(false);
        assert!(!demand.wanted(now));
        demand.changed();
        demand.changed();
        assert!(demand.wanted(now));
        demand.drawn(false);
        assert!(!demand.wanted(now));
        assert_eq!(demand.counts(), (2, 2));
    }

    #[test]
    fn a_frame_that_held_text_back_is_followed_by_one_that_sharpens_it() {
        let mut demand = FrameDemand::default();
        let now = Instant::now();
        demand.drawn(true);
        assert!(demand.wanted(now));
        demand.drawn(false);
        assert!(!demand.wanted(now));
    }

    #[test]
    fn a_window_that_gives_no_frame_is_tried_again_later_not_at_once() {
        let mut demand = FrameDemand::default();
        let now = Instant::now();
        demand.refused(now);
        assert!(!demand.wanted(now + Duration::from_millis(10)));
        assert!(demand.wanted(now + RETRY));
        let app = one_sticky().app().clone();
        // The loop wakes when the retry is due, though that is sooner than
        // an idle turn from the later time it asks.
        let later = now + Duration::from_millis(80);
        assert_eq!(demand.next_turn(&app, later), now + RETRY);
    }

    #[test]
    fn frames_keep_coming_for_a_moment_after_input_and_then_stop() {
        let mut demand = FrameDemand::default();
        let start = Instant::now();
        demand.drawn(false);
        demand.input();
        let mut frames = 0;
        for step in 0..120_u32 {
            let now = start + Duration::from_nanos(8_333_333) * step;
            if demand.wanted(now) {
                demand.drawn(false);
                frames += 1;
            }
        }
        // A quarter of a second of a 120 Hz second, and no more.
        assert!((29..=31).contains(&frames), "{frames} frames");
        assert!(!demand.wanted(start + Duration::from_secs(2)));
    }

    #[test]
    fn a_change_that_is_not_input_has_no_tail() {
        let mut demand = FrameDemand::default();
        let now = Instant::now();
        demand.drawn(false);
        demand.changed();
        assert!(demand.wanted(now));
        demand.drawn(false);
        assert!(!demand.wanted(now + Duration::from_millis(8)));
    }
}
