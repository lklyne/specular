//! Who is on the canvas: the agents with a cursor here, by name and
//! colour, and the one the camera follows. The `agents` state key, which a
//! mod draws as badges, and `camera.follow`, which a badge runs.

use serde_json::{Value, json};
use specular_agent::ThreadId;

use super::cursor::Cursor;
use crate::App;

/// One colour an agent, in the order agents first act: the cursor, its
/// chip and its badge.
pub const AGENT_COLORS: [[u8; 3]; 6] = [
    [59, 130, 246],
    [139, 92, 246],
    [236, 72, 153],
    [249, 115, 22],
    [34, 197, 94],
    [6, 182, 212],
];

/// The name the CLI gives itself when the caller set none.
const DEFAULT_CLIENT: &str = "specular-cli";

/// An agent the canvas shows a cursor for.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentBadge {
    /// The session the agent's calls carry.
    pub id: String,
    /// What to call it: its thread's title, the name it gave, or a number.
    pub name: String,
    /// Its colour, from [`AGENT_COLORS`].
    pub slot: usize,
    /// What it is doing, as its cursor's chip says.
    pub label: &'static str,
}

impl App {
    /// The agents with a cursor on the canvas shown, in the order they
    /// first acted. Listed while the setting hides the cursors too: hiding
    /// is drawing only.
    pub fn agents(&self) -> Vec<AgentBadge> {
        let now = self.session.now_ms;
        (self.session.presence.cursors.iter())
            .filter(|cursor| self.points_at_shown(cursor))
            .map(|cursor| AgentBadge {
                id: cursor.agent.clone(),
                name: name_of(self, cursor),
                slot: cursor.slot,
                label: if cursor.thinking(now) {
                    "thinking"
                } else {
                    cursor.label
                },
            })
            .collect()
    }

    /// The agent the camera follows, while it does.
    pub fn following(&self) -> Option<AgentBadge> {
        let agent = self.session.presence.following.as_deref()?;
        self.agents().into_iter().find(|badge| badge.id == agent)
    }
}

fn name_of(app: &App, cursor: &Cursor) -> String {
    let thread = app.threads.get(&ThreadId(cursor.agent.clone()));
    let given = app.session.presence.names.get(&cursor.agent);
    (thread.map(|thread| thread.title.clone()))
        .filter(|title| !title.trim().is_empty())
        .or_else(|| given.cloned())
        .unwrap_or_else(|| format!("Agent {}", cursor.slot + 1))
}

/// Keeps the name an agent gave itself, unless it is the CLI's default.
pub(crate) fn name(app: &mut App, agent: &str, name: Option<&str>) {
    let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) else {
        return;
    };
    if name != DEFAULT_CLIENT {
        (app.session.presence.names).insert(agent.to_owned(), name.to_owned());
    }
}

/// `#rrggbb` for colour `slot`.
pub fn agent_color(slot: usize) -> String {
    let [r, g, b] = AGENT_COLORS[slot % AGENT_COLORS.len()];
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// The `agents` state key: `{agents: [{id, name, color, label}], following}`.
/// No positions, so a cursor's travel wakes no mod.
pub(crate) fn agents_json(app: &App) -> Value {
    let agents: Vec<Value> = (app.agents().into_iter())
        .map(|badge| {
            json!({
                "id": badge.id,
                "name": badge.name,
                "color": agent_color(badge.slot),
                "label": badge.label,
            })
        })
        .collect();
    let following = app.following().map(|badge| badge.id);
    json!({ "agents": agents, "following": following })
}
