//! Flick inertia for touch-device pages.
//!
//! A left press on a touch page is a finger, and the page is told of it as
//! a touch and not as a mouse. When a touch ends while it is moving
//! Chromium starts a fling of its own, which an offscreen page never steps
//! and which then swallows every scroll sent after it. So the touch is not
//! ended there: the finger is kept on the page and moved on by a speed that
//! decays, and is lifted only once it has been still long enough for
//! Chromium to read no speed in it.
//!
//! A finger that pushes on where nothing is left to scroll stretches the
//! page, which springs back only when the finger lifts. So a glide stops
//! at the end of what it scrolls: when the page's own scrolling stops, or,
//! for a scroller inside the page, when the page says there is no room.
//!
//! ```text
//! press -> Drag --release or lift, moving--> Glide --slow--> Settle --> up
//!            ^                                 |                |
//!            +---------- press or finger back: the page is caught
//! ```

mod touch;

use self::touch::{Drag, Phase, Touch, glide_step};

use glam::Vec2;
use specular_core::{InputEvent, Modifiers, TouchEvent, TouchPhase};
use specular_doc::EntityId;

use crate::{App, Effect};

/// Scales the release speed, so the glide can be tuned to feel like a
/// finger's.
const MOUSE_BOOST: f32 = 1.0;
/// How far back from the last move the speed is read, in milliseconds.
const WINDOW_MS: u64 = 100;
/// A pointer still for this long before the release was held, not flicked.
/// A finger leaves the glass as it moves; a mouse button comes up a moment
/// after the hand has stopped, so this is longer than a touch screen's.
const HELD_MS: u64 = 100;
/// A swipe shorter than this, in CSS pixels, is a tap.
const MIN_TRAVEL: f32 = 10.0;
/// The slowest release, in CSS pixels a second, that glides.
const MIN_SPEED: f32 = 50.0;
/// The fastest glide, in CSS pixels a second.
const MAX_SPEED: f32 = 8000.0;
/// The glide ends when its speed falls under this.
const END_SPEED: f32 = 20.0;
/// What is left of the speed after a millisecond: iOS's normal deceleration
/// rate.
const DECAY_PER_MS: f32 = 0.998;
/// With the button still down, a pointer that has not moved for this long
/// has stopped, in milliseconds: two or three of a trackpad's reports.
const LIFT_MS: u64 = 24;
/// A pointer that was this fast, in CSS pixels a second, on its last move
/// before it stopped was not slowed to a stop: the finger left the
/// trackpad. A tap-and-drag keeps the button down for a while after that.
const LIFT_SPEED: f32 = 300.0;
/// How long the finger rests before it is lifted, in milliseconds: longer
/// than the 80 ms after which Chromium takes a touch to have stopped.
const SETTLE_MS: u64 = 96;
/// A glide whose page has scrolled under this finger and then not for this
/// long, in milliseconds, has run into the page's end. Chromium stretches a
/// page past its end for as long as a finger pushes there, and springs it
/// back when the finger lifts.
const END_MS: u64 = 100;
/// How often a glide the page does not report asks whether it still has
/// anything to scroll, in milliseconds.
const ROOM_MS: u64 = 100;
/// A minor axis slower than this share of the major one is zeroed, so a
/// vertical flick does not drift sideways.
const AXIS_LOCK: f32 = 0.5;

/// What a page is to be told of something the pointer did over it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Told {
    /// What a mouse did, at this place: the page is no touch device.
    Mouse(Vec2),
    /// What the page's finger did, and where it is.
    Finger(TouchPhase, Vec2),
    /// Nothing: a touch device has no pointer to hover with, and a finger
    /// that glides is not the pointer's to move.
    Nothing,
}

/// The effect that tells `page` what its finger did.
pub(crate) fn finger(page: EntityId, phase: TouchPhase, at: Vec2, modifiers: Modifiers) -> Effect {
    Effect::ForwardInput {
        page,
        event: InputEvent::Touch(TouchEvent {
            phase,
            position: at,
            modifiers,
        }),
    }
}

/// The finger a touch page has on it, from the press to the lift.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Flick(Option<Touch>);

/// Whether `page` is a touch device.
fn is_touch(app: &App, page: &EntityId) -> bool {
    app.page_device(page).touch
}

/// The finger on `page`, if it has one.
fn touch_on<'a>(app: &'a mut App, page: &EntityId) -> Option<&'a mut Touch> {
    (app.session.flick.0.as_mut()).filter(|touch| touch.page == *page)
}

/// A left press reached `page` at `local`, in its CSS pixels. A page
/// caught in its glide already has the finger on it.
pub(crate) fn press(app: &mut App, page: &EntityId, local: Vec2) -> Told {
    let now = app.session.now_ms;
    if let Some(touch) = touch_on(app, page) {
        touch.phase = Phase::Drag(Drag::new(touch.at - local, now, touch.at, true));
        return Told::Nothing;
    }
    if !is_touch(app, page) {
        return Told::Mouse(local);
    }
    app.session.flick.0 = Some(Touch {
        page: page.clone(),
        at: local,
        origin: local,
        scrolled_ms: None,
        ask_room_ms: 0,
        phase: Phase::Drag(Drag::new(Vec2::ZERO, now, local, false)),
    });
    Told::Finger(TouchPhase::Start, local)
}

/// The pointer is at `local` on `page`.
pub(crate) fn moved(app: &mut App, page: &EntityId, local: Vec2) -> Told {
    let now = app.session.now_ms;
    let touch_device = is_touch(app, page);
    let Some(touch) = touch_on(app, page) else {
        return if touch_device {
            Told::Nothing
        } else {
            Told::Mouse(local)
        };
    };
    match &mut touch.phase {
        Phase::Drag(drag) => {
            touch.at = local + drag.offset;
            drag.sample(now, touch.at);
            Told::Finger(TouchPhase::Move, touch.at)
        }
        // The finger is back on the trackpad within the time the button
        // stays down, and catches the page where it is.
        Phase::Glide {
            button_down: true, ..
        } => {
            touch.phase = Phase::Drag(Drag::new(touch.at - local, now, touch.at, true));
            Told::Nothing
        }
        Phase::Glide {
            button_down: false, ..
        }
        | Phase::Settle { .. } => Told::Nothing,
    }
}

/// The left button came up over `local` on the page that had its press. A
/// finger that was moving stays down: it glides on, or rests before it is
/// lifted.
pub(crate) fn release(app: &mut App, page: &EntityId, local: Vec2) -> Told {
    let now = app.session.now_ms;
    let Some(touch) = touch_on(app, page) else {
        return Told::Mouse(local);
    };
    match &mut touch.phase {
        Phase::Drag(drag) => {
            let still_ms = now.saturating_sub(drag.last_ms());
            let velocity = drag.velocity().filter(|_| still_ms < HELD_MS);
            if let Some(velocity) = velocity {
                touch.glide(velocity, now, now, false);
                return Told::Nothing;
            }
            // A finger that moved a moment ago rests before it is lifted.
            if (drag.travelled > 0.0 || drag.caught) && still_ms < SETTLE_MS {
                touch.phase = Phase::Settle {
                    until_ms: drag.last_ms() + SETTLE_MS,
                };
                return Told::Nothing;
            }
            let at = touch.at;
            app.session.flick.0 = None;
            Told::Finger(TouchPhase::End, at)
        }
        Phase::Glide { button_down, .. } => {
            *button_down = false;
            Told::Nothing
        }
        Phase::Settle { .. } => Told::Nothing,
    }
}

/// Stops a glide where it is: the user has gone on to something else. The
/// finger rests there, and is lifted once that is safe.
pub(crate) fn halt(app: &mut App) {
    let now = app.session.now_ms;
    if let Some(touch) = app.session.flick.0.as_mut()
        && matches!(touch.phase, Phase::Glide { .. })
    {
        touch.phase = Phase::Settle {
            until_ms: now + SETTLE_MS,
        };
    }
}

/// `page` said it had scrolled.
pub(crate) fn scrolled(app: &mut App, page: &EntityId) {
    let now = app.session.now_ms;
    if let Some(touch) = touch_on(app, page) {
        touch.scrolled_ms = Some(now);
    }
}

/// `page` said whether its gliding finger has anything to scroll. With
/// nothing, the finger would only stretch the page, so it stops there.
pub(crate) fn room(app: &mut App, page: &EntityId, room: bool) {
    if !room
        && app
            .session
            .flick
            .0
            .as_ref()
            .is_some_and(|touch| touch.page == *page)
    {
        halt(app);
    }
}

/// Moves a gliding finger on for the time since the last tick, lifts one
/// that has rested, and finds a finger that left the trackpad in mid swipe.
pub(crate) fn on_tick(app: &mut App, effects: &mut Vec<Effect>) {
    let now = app.session.now_ms;
    let modifiers = app.session.modifiers;
    let Some(touch) = app.session.flick.0.as_mut() else {
        return;
    };
    let page = touch.page.clone();
    match &mut touch.phase {
        Phase::Drag(drag) => {
            drag.still_ticks = drag.still_ticks.saturating_add(1);
            if drag.lifted(now)
                && let Some(velocity) = drag.velocity()
            {
                let last_ms = drag.last_ms();
                touch.glide(velocity, last_ms, now, true);
            }
        }
        Phase::Glide {
            velocity, last_ms, ..
        } => {
            // A page that scrolls under the finger shows its end by
            // stopping. One that says nothing is scrolling something inside
            // it, or nothing at all, and only it can tell which.
            if touch.scrolled_ms.is_none() && now >= touch.ask_room_ms {
                touch.ask_room_ms = now + ROOM_MS;
                effects.push(Effect::AskScrollRoom {
                    page: page.clone(),
                    at: touch.origin,
                    toward: *velocity,
                });
            }
            let dt_ms = now.saturating_sub(*last_ms);
            if dt_ms == 0 {
                return;
            }
            *last_ms = now;
            let (distance, left) = glide_step(*velocity, dt_ms as f32);
            *velocity = left;
            touch.at += distance;
            effects.push(finger(page, TouchPhase::Move, touch.at, modifiers));
            let at_end = (touch.scrolled_ms).is_some_and(|at| now.saturating_sub(at) >= END_MS);
            if left.length() < END_SPEED || at_end {
                touch.phase = Phase::Settle {
                    until_ms: now + SETTLE_MS,
                };
            }
        }
        Phase::Settle { until_ms } => {
            if now < *until_ms {
                return;
            }
            let at = touch.at;
            app.session.flick.0 = None;
            if app.document.entity(&page).is_some() {
                effects.push(finger(page, TouchPhase::End, at, modifiers));
            }
        }
    }
}

impl App {
    /// Whether a touch page has a finger on it, so the shell keeps ticking:
    /// the clock is what tells a finger has left the trackpad, steps a
    /// glide and lifts the finger at its end.
    pub fn flick_running(&self) -> bool {
        self.session.flick.0.is_some()
    }
}
