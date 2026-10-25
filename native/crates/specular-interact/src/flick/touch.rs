//! One finger on a touch page: its drag, its glide and its lift.

use glam::Vec2;
use specular_doc::EntityId;

use super::{
    AXIS_LOCK, DECAY_PER_MS, LIFT_MS, LIFT_SPEED, MAX_SPEED, MIN_SPEED, MIN_TRAVEL, MOUSE_BOOST,
    WINDOW_MS,
};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Touch {
    pub(super) page: EntityId,
    /// Where the finger is, in the page's CSS pixels.
    pub(super) at: Vec2,
    /// Where the finger came down, which settled what it scrolls.
    pub(super) origin: Vec2,
    /// When the page last said it had scrolled with this finger on it. A
    /// scroller inside the page says nothing as it scrolls.
    pub(super) scrolled_ms: Option<u64>,
    /// When a glide next asks the page whether it has anything to scroll.
    pub(super) ask_room_ms: u64,
    pub(super) phase: Phase,
}

impl Touch {
    pub(super) fn glide(&mut self, velocity: Vec2, last_ms: u64, now_ms: u64, button_down: bool) {
        // The wait for the page's end starts over with the glide.
        self.scrolled_ms = self.scrolled_ms.map(|_| now_ms);
        self.ask_room_ms = now_ms;
        self.phase = Phase::Glide {
            velocity,
            last_ms,
            button_down,
        };
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Phase {
    /// The button is down and the pointer moves the finger.
    Drag(Drag),
    /// The finger moves on by itself.
    Glide {
        /// CSS pixels a second.
        velocity: Vec2,
        last_ms: u64,
        /// Whether the button is still down: the finger left a trackpad
        /// whose tap-and-drag has not let go yet.
        button_down: bool,
    },
    /// The finger rests until it can be lifted with no speed.
    Settle { until_ms: u64 },
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Drag {
    /// From where the pointer is to where the finger is. A page caught in
    /// its glide is not where the pointer left it.
    pub(super) offset: Vec2,
    /// Where the finger was and when, newest last, one a millisecond.
    samples: Vec<(u64, Vec2)>,
    /// The length of the path so far.
    pub(super) travelled: f32,
    /// Whether the press caught a page in its glide, whose finger was
    /// moving until then.
    pub(super) caught: bool,
    /// How many ticks running found the finger where the last one left it.
    /// One is not a stop: the loop may only have been late, with the
    /// pointer's moves still waiting to be read.
    pub(super) still_ticks: u8,
}

impl Drag {
    pub(super) fn new(offset: Vec2, now_ms: u64, at: Vec2, caught: bool) -> Self {
        Self {
            offset,
            samples: vec![(now_ms, at)],
            travelled: 0.0,
            caught,
            still_ticks: 0,
        }
    }

    pub(super) fn sample(&mut self, now_ms: u64, at: Vec2) {
        let Some(&(last_ms, last)) = self.samples.last() else {
            self.samples.push((now_ms, at));
            return;
        };
        if at == last {
            return;
        }
        self.travelled += at.distance(last);
        self.still_ticks = 0;
        // Pointer events in one turn share a time; the latest wins.
        if last_ms == now_ms {
            self.samples.pop();
        }
        self.samples.push((now_ms, at));
        self.samples
            .retain(|&(at_ms, _)| at_ms + 2 * WINDOW_MS >= now_ms);
    }

    pub(super) fn last_ms(&self) -> u64 {
        self.samples.last().map_or(0, |&(at_ms, _)| at_ms)
    }

    /// Whether the finger left the trackpad in mid swipe: the pointer was
    /// fast on its last move and has not moved since.
    pub(super) fn lifted(&self, now_ms: u64) -> bool {
        let [.., (before_ms, before), (last_ms, last)] = self.samples[..] else {
            return false;
        };
        let span_ms = last_ms.saturating_sub(before_ms).max(1);
        self.still_ticks >= 2
            && now_ms.saturating_sub(last_ms) >= LIFT_MS
            && last.distance(before) / (span_ms as f32 / 1000.0) >= LIFT_SPEED
    }

    /// The velocity of the swipe's last moments, if it was fast and long
    /// enough to be a flick.
    pub(super) fn velocity(&self) -> Option<Vec2> {
        let &(last_ms, last) = self.samples.last()?;
        if self.travelled < MIN_TRAVEL {
            return None;
        }
        // Read back from the last move, not the release: the wait for the
        // button is not part of the flick. When the loop was late and the
        // window holds the last move alone, the one before it is the best
        // there is to measure from.
        let earlier = self.samples.len().checked_sub(2)?;
        let from = (self.samples.iter())
            .position(|&(at_ms, _)| at_ms + WINDOW_MS >= last_ms)?
            .min(earlier);
        let &(first_ms, first) = self.samples.get(from)?;
        let span_ms = last_ms.checked_sub(first_ms).filter(|&span| span > 0)?;
        let mut velocity = (last - first) / (span_ms as f32 / 1000.0);
        let speed = velocity.length();
        if speed < MIN_SPEED {
            return None;
        }
        velocity *= speed.min(MAX_SPEED) / speed * MOUSE_BOOST;
        let (major, minor) = if velocity.x.abs() >= velocity.y.abs() {
            (velocity.x.abs(), &mut velocity.y)
        } else {
            (velocity.y.abs(), &mut velocity.x)
        };
        if minor.abs() < major * AXIS_LOCK {
            *minor = 0.0;
        }
        Some(velocity)
    }
}

/// How far a finger at `velocity` moves in `dt_ms`, and the velocity it is
/// left with. The distance is the integral of the decaying speed, so a late
/// frame covers the same ground as the frames it replaced.
pub(super) fn glide_step(velocity: Vec2, dt_ms: f32) -> (Vec2, Vec2) {
    let remaining = DECAY_PER_MS.powf(dt_ms);
    let per_ms = -DECAY_PER_MS.ln();
    let distance = velocity / 1000.0 * ((1.0 - remaining) / per_ms);
    (distance, velocity * remaining)
}
