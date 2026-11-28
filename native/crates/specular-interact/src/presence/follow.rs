//! Following an agent: the camera keeps its cursor in the middle of what
//! the canvas is seen through, as Figma follows a collaborator, until the
//! person moves the camera themselves, presses Escape, or the agent goes.

use glam::Vec2;

use crate::{App, Event};

/// How quickly the camera catches up with the cursor: the time it takes to
/// close most of the gap.
const CATCH_UP_MS: f32 = 140.0;
/// How close, in screen pixels, counts as there.
const THERE_PX: f32 = 0.5;

/// Follows `agent`, or stops following with `None`. An agent with no
/// cursor here is not followed.
pub(crate) fn set(app: &mut App, agent: Option<String>) {
    let shown = agent.filter(|agent| app.agents().iter().any(|badge| badge.id == *agent));
    app.session.presence.following = shown;
}

/// Stops following. Whether anything was followed.
pub(crate) fn stop(app: &mut App) -> bool {
    app.session.presence.following.take().is_some()
}

/// The pan that puts the followed cursor in the middle.
fn target_pan(app: &App) -> Option<Vec2> {
    let agent = app.session.presence.following.as_deref()?;
    let index = app.session.presence.held(agent)?;
    let at = app.session.presence.cursors[index].resting(app)?.as_vec2();
    let camera = app.session.camera;
    Some(crate::viewport::centre(app) - at * camera.zoom)
}

/// Moves the camera toward the followed cursor by the time `elapsed_ms`
/// gave it. A followed agent that has gone is let go.
pub(crate) fn on_tick(app: &mut App, elapsed_ms: u64) {
    let Some(agent) = app.session.presence.following.clone() else {
        return;
    };
    if !app.agents().iter().any(|badge| badge.id == agent) {
        app.session.presence.following = None;
        return;
    }
    let Some(target) = target_pan(app) else {
        return;
    };
    let pan = app.session.camera.pan;
    let share = if app.session.motion.enabled() {
        #[expect(
            clippy::cast_precision_loss,
            reason = "a frame's milliseconds are small"
        )]
        let elapsed = elapsed_ms as f32;
        1.0 - (-elapsed / CATCH_UP_MS).exp()
    } else {
        1.0
    };
    let next = pan + (target - pan) * share;
    app.session.camera.pan = if (target - next).length() < THERE_PX {
        target
    } else {
        next
    };
}

/// Whether the camera is still on its way to the followed cursor.
pub(crate) fn moving(app: &App) -> bool {
    target_pan(app).is_some_and(|target| (target - app.session.camera.pan).length() >= THERE_PX)
}

/// Whether `event` is the person's own: a camera move it made ends a follow.
/// An agent's call and the clock are not.
pub(crate) fn by_person(event: &Event) -> bool {
    matches!(
        event,
        Event::Pointer(_)
            | Event::Wheel(_)
            | Event::Pinch { .. }
            | Event::Key(_)
            | Event::Action(_)
    )
}
