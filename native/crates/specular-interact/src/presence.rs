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

use glam::DVec2;
use specular_doc::EntityId;

use crate::{App, Effect};

/// The travel time of a move too short to see, in milliseconds.
const TRAVEL_BASE_MS: f64 = 90.0;
/// What each screen pixel of distance adds to it.
const TRAVEL_MS_PER_PX: f64 = 0.35;
/// The longest travel, however far.
const TRAVEL_MAX_MS: f64 = 320.0;
/// The screen distance, in pixels, under which a cursor is already there.
const NEGLIGIBLE_PX: f64 = 2.0;
/// How long a cursor rests on its target before the act, so the arrival is
/// seen as one.
const SETTLE_MS: u64 = 40;
/// How long a cursor stays after its session's last act.
const IDLE_MS: u64 = 10_000;

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
}

#[derive(Debug, Clone, PartialEq)]
struct Cursor {
    agent: String,
    page: EntityId,
    /// The target, in the CSS pixels of the page's document, so the cursor
    /// stays on what it points at when the page scrolls.
    anchor: DVec2,
    /// Where the travel began, in canvas space.
    from: DVec2,
    /// The clock at the first tick after the travel began.
    started_ms: Option<u64>,
    travel_ms: u64,
    label: &'static str,
    /// The act waiting for the arrival.
    waiting: Option<u64>,
    /// The clock at the latest act, once a tick has seen it.
    acted_ms: Option<u64>,
    slot: usize,
}

impl Cursor {
    /// How much of the travel is done at `now_ms`, eased out.
    fn progress(&self, now_ms: u64) -> f64 {
        let Some(started) = self.started_ms else {
            return 0.0;
        };
        if self.travel_ms == 0 {
            return 1.0;
        }
        let linear = (now_ms.saturating_sub(started) as f64 / self.travel_ms as f64).min(1.0);
        1.0 - (1.0 - linear).powi(3)
    }

    /// The target in the page's viewport CSS pixels, as it is scrolled now.
    fn in_viewport(&self, app: &App) -> DVec2 {
        self.anchor - app.page_scroll(&self.page)
    }

    fn position(&self, app: &App) -> Option<DVec2> {
        let target = app
            .page_placement(&self.page)?
            .to_canvas(self.in_viewport(app));
        Some(self.from.lerp(target, self.progress(app.session.now_ms)))
    }

    /// Whether the cursor rests on something the page has scrolled out of
    /// its viewport.
    fn scrolled_away(&self, app: &App) -> bool {
        let Some(placement) = app.page_placement(&self.page) else {
            return false;
        };
        let at = self.in_viewport(app);
        let (width, height) = (
            f64::from(placement.viewport.width),
            f64::from(placement.viewport.height),
        );
        let inside = (0.0..=width).contains(&at.x) && (0.0..=height).contains(&at.y);
        self.arrived(app.session.now_ms) && !inside
    }

    fn arrived(&self, now_ms: u64) -> bool {
        (self.started_ms).is_some_and(|started| now_ms >= started + self.travel_ms + SETTLE_MS)
    }
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
    let held = (app.session.presence.cursors.iter()).position(|cursor| cursor.agent == agent);
    // A cursor already on its way to this point keeps going: the act waits
    // for what is left of that travel and no new one starts.
    let anchor = act.at + app.page_scroll(&act.page);
    if let Some(index) = held {
        let cursor = &mut app.session.presence.cursors[index];
        let same = cursor.page == act.page && (cursor.anchor - anchor).length() < NEGLIGIBLE_PX;
        if same && !cursor.arrived(app.session.now_ms) {
            effects.extend((cursor.waiting.take()).map(|ticket| Effect::AgentGo { ticket }));
            cursor.waiting = act.ticket;
            cursor.label = label_of(&act.action);
            return;
        }
    }
    let from = held
        .and_then(|index| app.session.presence.cursors[index].position(app))
        .unwrap_or_else(|| placement.to_canvas(DVec2::ZERO));
    let distance_px = (target - from).length() * f64::from(app.session.camera.zoom);
    let travel_ms = if !app.session.motion.enabled() || distance_px < NEGLIGIBLE_PX {
        0
    } else {
        (TRAVEL_BASE_MS + TRAVEL_MS_PER_PX * distance_px).min(TRAVEL_MAX_MS) as u64
    };
    let presence = &mut app.session.presence;
    let slot = held.map_or(presence.seen, |index| presence.cursors[index].slot);
    let mut cursor = Cursor {
        agent: agent.to_owned(),
        page: act.page,
        anchor,
        from,
        started_ms: None,
        travel_ms,
        label: label_of(&act.action),
        waiting: act.ticket,
        acted_ms: None,
        slot,
    };
    if travel_ms == 0
        && let Some(ticket) = cursor.waiting.take()
    {
        effects.push(Effect::AgentGo { ticket });
    }
    if let Some(index) = held {
        // An act this one overtakes was given up on by whoever asked.
        if let Some(ticket) = presence.cursors[index].waiting {
            effects.push(Effect::AgentGo { ticket });
        }
        presence.cursors[index] = cursor;
    } else {
        presence.seen += 1;
        presence.cursors.push(cursor);
    }
}

/// Moves the cursors with the clock, answers the acts whose cursor has
/// arrived and drops the cursors of sessions that have gone quiet.
pub(crate) fn on_tick(app: &mut App, effects: &mut Vec<Effect>) {
    let now = app.session.now_ms;
    let presence = &mut app.session.presence;
    if presence.cursors.is_empty() {
        return;
    }
    let mut changed = false;
    for cursor in &mut presence.cursors {
        changed |= !cursor.arrived(now);
        cursor.started_ms.get_or_insert(now);
        cursor.acted_ms.get_or_insert(now);
        if cursor.arrived(now)
            && let Some(ticket) = cursor.waiting.take()
        {
            cursor.acted_ms = Some(now);
            effects.push(Effect::AgentGo { ticket });
        }
    }
    let before = presence.cursors.len();
    (presence.cursors).retain(|cursor| {
        cursor.waiting.is_some()
            || (cursor.acted_ms).is_none_or(|acted| now.saturating_sub(acted) < IDLE_MS)
    });
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
        (self.session.presence.cursors.iter())
            .filter(|cursor| {
                (self.document.entity(&cursor.page))
                    .is_some_and(|entity| crate::shown_rect(self, entity).is_some())
                    && !cursor.scrolled_away(self)
            })
            .filter_map(|cursor| {
                Some(AgentCursor {
                    at: cursor.position(self)?,
                    label: cursor.label,
                    slot: cursor.slot,
                })
            })
            .collect()
    }

    /// Whether an agent cursor is on its way, so the shell keeps ticking
    /// and drawing until it lands.
    pub fn agent_cursor_travelling(&self) -> bool {
        let now = self.session.now_ms;
        (self.session.presence.cursors.iter()).any(|cursor| !cursor.arrived(now))
    }

    /// Counts the ticks that moved an agent cursor or took one away, so a
    /// shell can tell the clock alone changed what is drawn.
    pub fn agent_cursor_epoch(&self) -> u64 {
        self.session.presence.epoch
    }
}
