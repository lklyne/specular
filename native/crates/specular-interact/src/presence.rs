//! Agent cursors: where each agent session points, and the hold that lets a
//! cursor arrive before its page reacts (ADR 0029).
//!
//! ```text
//! agent-browser --Specular.willAct--> Event::AgentAct --> the cursor travels
//! agent-browser <------reply--------- Effect::AgentGo <-- it has arrived
//! ```
//!
//! agent-browser resolves its target, says where it is about to act and
//! waits for the answer. The answer is sent when the cursor is drawn at
//! that point, so the act never lands with the cursor still on its way and
//! nothing here guesses how long a travel takes to show.
//!
//! A read is announced the same way and answered at once: the cursor goes
//! to what the agent looked at and nothing waits for it, so the act that
//! often follows on the same element starts with the cursor there.
//!
//! The canvas verbs move the same cursor ([`canvas`]): an API call that
//! changes the canvas sends it to what changed, after the change.

pub(crate) mod canvas;
mod cursor;

use glam::DVec2;
use specular_doc::EntityId;

use self::cursor::{Anchor, Cursor, IDLE_MS, NEGLIGIBLE_PX, RIPPLE_MS, travel_ms};
use crate::{App, Effect};

/// agent-browser is about to act on a page and is waiting to be told to.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentAct {
    /// What the answer, [`Effect::AgentGo`], carries back. `None` for a
    /// read, which was answered before the cursor set off.
    pub ticket: Option<u64>,
    /// The page entity.
    pub page: EntityId,
    /// The agent-browser session, which the CLI names `<agent>:<page>`.
    pub session: String,
    /// The verb, as the CLI spells it.
    pub action: String,
    /// Where, in the page's viewport CSS pixels.
    pub at: DVec2,
}

/// One agent cursor as a frame draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentCursor {
    /// The tip, in canvas space.
    pub at: DVec2,
    /// What it is doing, as a lowercase gerund.
    pub label: &'static str,
    /// Which of the cursor colours it has. Cursors are numbered as their
    /// sessions first act.
    pub slot: usize,
    /// How opaque it is, from 0 to 1: it fades out once its agent is gone.
    pub alpha: f32,
    /// Where the tip just was on its way, nearest first, in canvas space.
    pub trail: Vec<DVec2>,
    /// How far the ring of its landing has spread, from 0 to 1.
    pub ripple: Option<f32>,
}

/// Every agent cursor of the session.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Presence {
    cursors: Vec<Cursor>,
    /// How many sessions have had a cursor.
    seen: usize,
    /// Counts the ticks that moved a cursor or took one away.
    epoch: u64,
}

impl Presence {
    fn held(&self, agent: &str) -> Option<usize> {
        (self.cursors.iter()).position(|cursor| cursor.agent == agent)
    }

    /// Puts `cursor` in the place of `agent`'s, keeping its colour, or adds
    /// it with the next colour. An act the old one held was given up on by
    /// whoever asked, so it is let go.
    fn put(&mut self, mut cursor: Cursor, effects: &mut Vec<Effect>) {
        if let Some(index) = self.held(&cursor.agent) {
            let old = &mut self.cursors[index];
            cursor.slot = old.slot;
            effects.extend((old.waiting.take()).map(|ticket| Effect::AgentGo { ticket }));
            *old = cursor;
        } else {
            cursor.slot = self.seen;
            self.seen += 1;
            self.cursors.push(cursor);
        }
    }
}

/// The agent a session belongs to: the CLI runs one agent-browser session
/// for each page an agent drives, and one cursor follows the agent.
fn agent_of(session: &str) -> &str {
    session.rsplit_once(':').map_or(session, |(agent, _)| agent)
}

fn label_of(action: &str) -> &'static str {
    match action {
        "click" | "dblclick" | "tap" => "clicking",
        "hover" => "pointing",
        "read" => "reading",
        "drag" | "drop" => "dragging",
        "press" => "pressing",
        "fill" | "type" => "typing",
        "select" => "selecting",
        "check" => "checking",
        "uncheck" => "unchecking",
        "focus" => "focusing",
        "scrollintoview" => "scrolling",
        _ => "working",
    }
}

/// Starts the cursor of `act`'s agent toward its target. A target that is
/// not on the canvas shown, or a cursor already there, is answered at once.
pub(crate) fn on_act(app: &mut App, act: AgentAct, effects: &mut Vec<Effect>) {
    let shown = (app.document.entity(&act.page))
        .is_some_and(|entity| crate::shown_rect(app, entity).is_some());
    let Some(placement) = app.page_placement(&act.page).filter(|_| shown) else {
        effects.extend(act.ticket.map(|ticket| Effect::AgentGo { ticket }));
        return;
    };
    let target = placement.to_canvas(act.at);
    let agent = agent_of(&act.session);
    let label = label_of(&act.action);
    let held = app.session.presence.held(agent);
    // A cursor already on its way to this point keeps going: the act waits
    // for what is left of that travel and no new one starts.
    let css = act.at + app.page_scroll(&act.page);
    if let Some(index) = held {
        let now = app.session.now_ms;
        let cursor = &mut app.session.presence.cursors[index];
        let same = matches!(&cursor.anchor, Anchor::Page { page, css: at }
            if *page == act.page && (*at - css).length() < NEGLIGIBLE_PX);
        if same && !cursor.arrived(now) {
            effects.extend((cursor.waiting.take()).map(|ticket| Effect::AgentGo { ticket }));
            cursor.waiting = act.ticket;
            cursor.label = label;
            cursor.ripples = label == "clicking";
            return;
        }
    }
    let anchor = Anchor::Page {
        page: act.page,
        css,
    };
    let mut cursor = if let Some(index) = held {
        let mut cursor = app.session.presence.cursors[index].clone();
        cursor.head_to(app, anchor, label);
        cursor.stops.clear();
        cursor
    } else {
        let from = placement.to_canvas(DVec2::ZERO);
        let mut cursor = Cursor::new(agent, anchor, from, label);
        cursor.travel_ms = travel_ms(app, from, target);
        cursor
    };
    cursor.acted_ms = None;
    cursor.ripples = label == "clicking";
    cursor.waiting = act.ticket;
    if cursor.travel_ms == 0
        && let Some(ticket) = cursor.waiting.take()
    {
        effects.push(Effect::AgentGo { ticket });
    }
    app.session.presence.put(cursor, effects);
}

/// Moves the cursors with the clock: answers the acts whose cursor has
/// arrived, sends a canvas act on to its next stop and drops the cursors
/// of sessions that have gone quiet.
pub(crate) fn on_tick(app: &mut App, effects: &mut Vec<Effect>) {
    let now = app.session.now_ms;
    if app.session.presence.cursors.is_empty() {
        return;
    }
    let mut changed = false;
    let mut onward = Vec::new();
    for (index, cursor) in app.session.presence.cursors.iter_mut().enumerate() {
        changed |= !cursor.arrived(now) || cursor.ripple(now).is_some();
        cursor.started_ms.get_or_insert(now);
        cursor.acted_ms.get_or_insert(now);
        if cursor.arrived(now) && cursor.landed_ms.is_none() {
            cursor.landed_ms = Some(now);
            cursor.acted_ms = Some(now);
            effects.extend((cursor.waiting.take()).map(|ticket| Effect::AgentGo { ticket }));
        }
        let thinking = cursor.thinking(now);
        changed |= thinking != cursor.drawn_thinking;
        cursor.drawn_thinking = thinking;
        let dwelt = (cursor.landed_ms).is_some_and(|landed| now >= landed + canvas::STOP_DWELL_MS);
        if dwelt && !cursor.stops.is_empty() {
            onward.push(index);
        }
    }
    for index in onward {
        let mut cursor = app.session.presence.cursors[index].clone();
        if let Some(stop) = cursor.stops.pop_front() {
            let canvas = app.space.active().id.clone();
            cursor.head_to(
                app,
                Anchor::Canvas {
                    canvas,
                    at: stop.at,
                },
                stop.label,
            );
            cursor.started_ms = Some(now);
        }
        app.session.presence.cursors[index] = cursor;
        changed = true;
    }
    let motion = app.session.motion.enabled();
    let presence = &mut app.session.presence;
    let before = presence.cursors.len();
    (presence.cursors).retain(|cursor| {
        cursor.waiting.is_some()
            || !cursor.stops.is_empty()
            || (cursor.acted_ms).is_none_or(|acted| now.saturating_sub(acted) < IDLE_MS)
    });
    // A thinking cursor wanders and a quiet one fades, both on the clock.
    changed |= (presence.cursors.iter())
        .any(|cursor| (motion && cursor.thinking(now)) || cursor.alpha(now) < 1.0);
    if changed || presence.cursors.len() != before {
        presence.epoch += 1;
    }
}

impl App {
    /// The agent cursors on the canvas shown, in the order their sessions
    /// first acted. None while the setting hides them; the agents act all
    /// the same.
    pub fn agent_cursors(&self) -> Vec<AgentCursor> {
        if self.settings.hidden_cursors.agent {
            return Vec::new();
        }
        let now = self.session.now_ms;
        (self.session.presence.cursors.iter())
            .filter(|cursor| self.points_at_shown(cursor) && !cursor.scrolled_away(self))
            .filter_map(|cursor| {
                Some(AgentCursor {
                    at: cursor.position(self)?,
                    label: if cursor.thinking(now) {
                        "thinking"
                    } else {
                        cursor.label
                    },
                    slot: cursor.slot,
                    alpha: cursor.alpha(now),
                    trail: cursor.trail(self),
                    ripple: cursor.ripple(now),
                })
            })
            .collect()
    }

    fn points_at_shown(&self, cursor: &Cursor) -> bool {
        match &cursor.anchor {
            Anchor::Page { page, .. } => (self.document.entity(page))
                .is_some_and(|entity| crate::shown_rect(self, entity).is_some()),
            Anchor::Canvas { canvas, .. } => *canvas == self.space.active().id,
        }
    }

    /// Whether an agent cursor is on its way or rippling, so the shell keeps
    /// ticking and drawing a frame at a time until it settles.
    pub fn agent_cursor_travelling(&self) -> bool {
        let now = self.session.now_ms;
        (self.session.presence.cursors.iter()).any(|cursor| {
            !cursor.arrived(now)
                || !cursor.stops.is_empty()
                || cursor
                    .landed_ms
                    .is_some_and(|landed| cursor.ripples && now.saturating_sub(landed) < RIPPLE_MS)
        })
    }

    /// Counts the ticks that moved an agent cursor or took one away, so a
    /// shell can tell the clock alone changed what is drawn.
    pub fn agent_cursor_epoch(&self) -> u64 {
        self.session.presence.epoch
    }
}
