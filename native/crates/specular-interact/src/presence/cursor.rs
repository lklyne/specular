//! One agent cursor: where it points, the curved travel to there, the
//! ripple it lands with and the drift it rests in.

use std::collections::VecDeque;

use glam::DVec2;
use specular_doc::{EntityId, Rect};

use crate::{App, CanvasId};

/// The travel time of a move too short to see, in milliseconds.
const TRAVEL_BASE_MS: f64 = 90.0;
/// What each screen pixel of distance adds to it.
const TRAVEL_MS_PER_PX: f64 = 0.35;
/// The longest travel, however far.
const TRAVEL_MAX_MS: f64 = 320.0;
/// The longest travel while reading, which hops from item to item quickly.
const READ_TRAVEL_MAX_MS: f64 = 120.0;
/// How long an item the cursor read stays lit.
pub(super) const GLOW_MS: u64 = 700;
/// The screen distance, in pixels, under which a cursor is already there.
pub(super) const NEGLIGIBLE_PX: f64 = 2.0;
/// How long a cursor rests on its target before the act, so the arrival is
/// seen as one.
const SETTLE_MS: u64 = 40;
/// How far the path bows out of the straight line, as a share of its
/// length, so a travel reads as a hand's arc and not a slide.
const BEND: f64 = 0.16;
/// How long a landing's ripple spreads.
pub(super) const RIPPLE_MS: u64 = 450;
/// How long after its last act a cursor reads as thinking.
pub(super) const THINKING_MS: u64 = 3_000;
/// How long a cursor stays after its session's last act.
pub(super) const IDLE_MS: u64 = 30_000;
/// The last stretch of that, over which it fades out.
pub(super) const FADE_MS: u64 = 1_000;
/// The trail's dots, each this much of the travel behind the one before.
const TRAIL_DOTS: usize = 6;
const TRAIL_STEP: f64 = 0.06;
/// The wander of a thinking cursor, in screen pixels, and how long one
/// lazy loop takes. A cursor that last read wanders further, as if
/// scanning what it read.
const DRIFT: (f64, f64) = (2.5, 5_200.0);
const READING_DRIFT: (f64, f64) = (4.0, 4_200.0);
/// How long the drift takes to grow from nothing.
const DRIFT_RAMP_MS: f64 = 450.0;

/// What a cursor points at.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Anchor {
    /// A point in a page's document, in its CSS pixels, so the cursor stays
    /// on what it points at when the page scrolls.
    Page { page: EntityId, css: DVec2 },
    /// A point on a canvas.
    Canvas { canvas: CanvasId, at: DVec2 },
}

/// A point a canvas act visits after the one the cursor is on.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Stop {
    pub(super) at: DVec2,
    pub(super) label: &'static str,
    /// The item it reads, lit once the cursor lands on it.
    pub(super) item: Option<Rect>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Cursor {
    pub(super) agent: String,
    pub(super) anchor: Anchor,
    /// Where the travel began, in canvas space.
    pub(super) from: DVec2,
    /// Which way the path bows: 1 or -1.
    pub(super) side: f64,
    /// The clock at the first tick after the travel began.
    pub(super) started_ms: Option<u64>,
    pub(super) travel_ms: u64,
    pub(super) label: &'static str,
    /// The act waiting for the arrival.
    pub(super) waiting: Option<u64>,
    /// The clock at the latest act or landing, once a tick has seen it.
    pub(super) acted_ms: Option<u64>,
    /// The clock when this travel landed.
    pub(super) landed_ms: Option<u64>,
    /// Whether the landing ripples: a click, or a canvas edit.
    pub(super) ripples: bool,
    /// What the act visits once this travel has landed.
    pub(super) stops: VecDeque<Stop>,
    pub(super) slot: usize,
    pub(super) seed: u32,
    /// The item the travel reads, lit when it lands.
    pub(super) item: Option<Rect>,
    /// The items it has read lately, and when.
    pub(super) read: Vec<(Rect, u64)>,
    /// Whether the last frame drew it thinking, so the turn to "thinking"
    /// is drawn even with no drift to draw it.
    pub(super) drawn_thinking: bool,
}

impl Cursor {
    /// A cursor for `agent` on its way from `from` to `anchor`.
    pub(super) fn new(agent: &str, anchor: Anchor, from: DVec2, label: &'static str) -> Self {
        let seed = seed_of(agent);
        Self {
            agent: agent.to_owned(),
            anchor,
            from,
            side: if seed.is_multiple_of(2) { 1.0 } else { -1.0 },
            started_ms: None,
            travel_ms: 0,
            label,
            waiting: None,
            acted_ms: None,
            landed_ms: None,
            ripples: false,
            stops: VecDeque::new(),
            slot: 0,
            seed,
            item: None,
            read: Vec::new(),
            drawn_thinking: false,
        }
    }

    /// The target in canvas space, as the page is placed and scrolled now.
    /// `None` when it is on a page that is gone or a canvas not shown.
    pub(super) fn target(&self, app: &App) -> Option<DVec2> {
        match &self.anchor {
            Anchor::Page { page, css } => Some(
                app.page_placement(page)?
                    .to_canvas(*css - app.page_scroll(page)),
            ),
            Anchor::Canvas { canvas, at } => (*canvas == app.space.active().id).then_some(*at),
        }
    }

    /// How much of the travel is done at `now_ms`, eased in and out.
    fn progress(&self, now_ms: u64) -> f64 {
        let Some(started) = self.started_ms else {
            return 0.0;
        };
        if self.travel_ms == 0 {
            return 1.0;
        }
        let t = (now_ms.saturating_sub(started) as f64 / self.travel_ms as f64).min(1.0);
        if t < 0.5 {
            4.0 * t * t * t
        } else {
            1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
        }
    }

    /// The point `share` of the way along the bowed path to `target`.
    fn along(&self, target: DVec2, share: f64) -> DVec2 {
        let span = target - self.from;
        let control = self.from.midpoint(target) + span.perp() * BEND * self.side;
        let rest = 1.0 - share;
        self.from * rest * rest + control * 2.0 * rest * share + target * share * share
    }

    /// Where the tip is on its path now, with no drift: what a follow
    /// keeps in view.
    pub(super) fn resting(&self, app: &App) -> Option<DVec2> {
        Some(self.along(self.target(app)?, self.progress(app.session.now_ms)))
    }

    /// The items read lately, each with how lit it still is, from 1 to 0.
    pub(super) fn glow(&self, now_ms: u64) -> Vec<(Rect, f32)> {
        (self.read.iter())
            .filter_map(|(rect, at)| {
                let spent = now_ms.saturating_sub(*at);
                (spent < GLOW_MS).then(|| (*rect, 1.0 - spent as f32 / GLOW_MS as f32))
            })
            .collect()
    }

    /// Where the tip is drawn now, drift included.
    pub(super) fn position(&self, app: &App) -> Option<DVec2> {
        let now = app.session.now_ms;
        let at = self.along(self.target(app)?, self.progress(now));
        Some(at + self.drift(app) / f64::from(app.session.camera.zoom))
    }

    /// The trail behind a travelling tip, nearest first.
    pub(super) fn trail(&self, app: &App) -> Vec<DVec2> {
        let now = app.session.now_ms;
        if self.arrived(now) || !app.session.motion.enabled() {
            return Vec::new();
        }
        let Some(target) = self.target(app) else {
            return Vec::new();
        };
        let progress = self.progress(now);
        (1..=TRAIL_DOTS)
            .map(|dot| progress - TRAIL_STEP * dot as f64)
            .take_while(|share| *share > 0.0)
            .map(|share| self.along(target, share))
            .collect()
    }

    /// Whether the cursor rests on something its page has scrolled out of
    /// its viewport.
    pub(super) fn scrolled_away(&self, app: &App) -> bool {
        let Anchor::Page { page, css } = &self.anchor else {
            return false;
        };
        let Some(placement) = app.page_placement(page) else {
            return false;
        };
        let at = *css - app.page_scroll(page);
        let (width, height) = (
            f64::from(placement.viewport.width),
            f64::from(placement.viewport.height),
        );
        let inside = (0.0..=width).contains(&at.x) && (0.0..=height).contains(&at.y);
        self.arrived(app.session.now_ms) && !inside
    }

    pub(super) fn arrived(&self, now_ms: u64) -> bool {
        (self.started_ms).is_some_and(|started| now_ms >= started + self.travel_ms + SETTLE_MS)
    }

    /// Whether the cursor sits between acts: landed, with nothing waiting
    /// and nothing left to visit, for a while.
    pub(super) fn thinking(&self, now_ms: u64) -> bool {
        self.resting_ms(now_ms)
            .is_some_and(|rest| rest >= THINKING_MS)
    }

    /// How long the cursor has rested since its last act.
    fn resting_ms(&self, now_ms: u64) -> Option<u64> {
        if self.waiting.is_some() || !self.stops.is_empty() || !self.arrived(now_ms) {
            return None;
        }
        Some(now_ms.saturating_sub(self.acted_ms?))
    }

    /// How far a landing ripple has spread, from 0 to 1.
    pub(super) fn ripple(&self, now_ms: u64) -> Option<f32> {
        let landed = self.landed_ms.filter(|_| self.ripples)?;
        let spent = now_ms.saturating_sub(landed);
        (spent < RIPPLE_MS).then(|| spent as f32 / RIPPLE_MS as f32)
    }

    /// How opaque the cursor is: whole until its last second, when it fades.
    pub(super) fn alpha(&self, now_ms: u64) -> f32 {
        let Some(rest) = self.resting_ms(now_ms) else {
            return 1.0;
        };
        let left = IDLE_MS.saturating_sub(rest);
        (left as f32 / FADE_MS as f32).min(1.0)
    }

    /// The thinking wander, in screen pixels. Never on the way to a target,
    /// so it does not fight a real move.
    fn drift(&self, app: &App) -> DVec2 {
        let now = app.session.now_ms;
        if !app.session.motion.enabled() || !self.thinking(now) {
            return DVec2::ZERO;
        }
        let since = self
            .resting_ms(now)
            .unwrap_or(0)
            .saturating_sub(THINKING_MS) as f64;
        let (radius, period) = if self.label == "reading" {
            READING_DRIFT
        } else {
            DRIFT
        };
        let x = wander(self.seed ^ 0x11, since, period);
        let y = wander(self.seed ^ 0x1f, since, period * 1.15);
        let ramp = (since / DRIFT_RAMP_MS).min(1.0);
        (DVec2::new(x, y) * radius * ramp).clamp_length_max(radius)
    }

    /// Starts a travel from where the tip is now to `anchor`.
    pub(super) fn head_to(&mut self, app: &App, anchor: Anchor, label: &'static str) {
        let from = self.position(app).unwrap_or(self.from);
        self.from = from;
        self.anchor = anchor;
        self.label = label;
        self.started_ms = None;
        self.landed_ms = None;
        self.travel_ms = self
            .target(app)
            .map_or(0, |to| travel_ms(app, from, to, label));
        self.side = -self.side;
        self.item = None;
    }
}

/// How long a travel from `from` to `to` takes to show.
/// Reading hops quicker than an act travels.
pub(super) fn travel_ms(app: &App, from: DVec2, to: DVec2, label: &str) -> u64 {
    let distance_px = (to - from).length() * f64::from(app.session.camera.zoom);
    if !app.session.motion.enabled() || distance_px < NEGLIGIBLE_PX {
        return 0;
    }
    let most = if label == "reading" {
        READ_TRAVEL_MAX_MS
    } else {
        TRAVEL_MAX_MS
    };
    (TRAVEL_BASE_MS + TRAVEL_MS_PER_PX * distance_px).min(most) as u64
}

/// Two sines a golden ratio apart, so the wander never shows a short loop,
/// with no state between frames.
fn wander(seed: u32, since_ms: f64, period_ms: f64) -> f64 {
    let phase = f64::from(seed % 1000) / 1000.0 * std::f64::consts::TAU;
    let t = since_ms / period_ms * std::f64::consts::TAU;
    (t + phase).sin() * 0.7 + (t * 1.618 + phase * 1.3 + 1.7).sin() * 0.3
}

/// A stable number for an agent, so two cursors do not wander in step.
fn seed_of(agent: &str) -> u32 {
    agent.bytes().fold(2_166_136_261_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(16_777_619)
    })
}
