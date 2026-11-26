//! Pages an agent is driving stay awake (ADR 0053).
//!
//! A page no one can see stops painting: off the camera, or in a window
//! that is minimised or covered. A hidden page runs no animation frames,
//! throttles its timers and makes no frames, so an agent driving it through
//! its devtools channel would wait on it and be shown a stale picture. Each
//! message an agent sends a page, and each act it announces, keeps that page
//! in use for [`AWAKE_MS`]; the shell paints a page in use whatever is
//! shown. Then it falls back to the usual rules.

use std::collections::HashMap;

use specular_doc::EntityId;

use crate::App;

/// How long a page stays awake after an agent last drove it.
const AWAKE_MS: u64 = 5_000;

/// Until when each driven page stays awake, by the session clock.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct AgentWake {
    until: HashMap<EntityId, u64>,
}

/// An agent drove `page`: it stays awake for [`AWAKE_MS`] from now.
pub(crate) fn on_drive(app: &mut App, page: EntityId) {
    let until = app.session.now_ms + AWAKE_MS;
    app.session.agent_wake.until.insert(page, until);
}

/// Forgets the pages whose time is up.
pub(crate) fn on_tick(app: &mut App) {
    let now = app.session.now_ms;
    app.session.agent_wake.until.retain(|_, until| *until > now);
}

impl App {
    /// Whether an agent has driven `page` lately, so it paints at the
    /// display's rate whether or not anyone can see it.
    pub fn agent_driving(&self, page: &EntityId) -> bool {
        (self.session.agent_wake.until.get(page)).is_some_and(|until| *until > self.session.now_ms)
    }

    /// How long until the next driven page falls back to the usual rules,
    /// so the loop turns then to let it sleep.
    pub fn agent_wake_lapses_in_ms(&self) -> Option<u64> {
        let now = self.session.now_ms;
        (self.session.agent_wake.until.values())
            .map(|until| until.saturating_sub(now))
            .min()
    }
}
