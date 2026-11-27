//! A mod tool's stroke: the points the pointer passed through while the tool
//! was armed, which `update` gathers and the mod reads as the `tool` state
//! key.
//!
//! The script never runs inside `update`. The mod renders when the key moves,
//! which is at most once a frame however many moves arrived, and it sees the
//! whole stroke so far each time, so no point is lost to the coalescing. It
//! answers with a preview (`tool.preview`), drawn a frame behind the
//! pointer, and on release commits with `item.add`, one undo step.

use glam::DVec2;
use serde::Serialize;

use crate::{App, Gesture};

/// A mod tool, as an index into the installed tools in toolbar order
/// ([`ModCatalog::tools`](super::ModCatalog::tools)). Small and `Copy`, so
/// [`Tool`](crate::Tool) stays `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ModToolId(pub u16);

/// Where a stroke is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StrokePhase {
    /// The button is down.
    Down,
    /// The button came up: the mod commits now.
    Up,
}

/// The stroke of the armed mod tool.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModStroke {
    /// The tool, as `<mod>/<tool>`.
    pub tool: String,
    /// Counts up with each press, so a mod commits each stroke once.
    pub number: u64,
    /// Where it is.
    pub phase: StrokePhase,
    /// The points so far in canvas units, `[x, y]` each, the press first.
    pub points: Vec<[f64; 2]>,
}

/// Points nearer than this to the last one, in canvas units at 100%, are
/// not added: a slow drag would otherwise send thousands.
const SPACING: f64 = 1.0;

fn point(at: DVec2) -> [f64; 2] {
    [
        (at.x * 100.0).round() / 100.0,
        (at.y * 100.0).round() / 100.0,
    ]
}

/// A press with the mod tool `tool` at canvas point `at`.
pub(crate) fn begin_stroke(app: &mut App, tool: String, at: DVec2) {
    let mods = &mut app.mods;
    let number = mods.stroke.as_ref().map_or(1, |stroke| stroke.number + 1);
    mods.preview.clear();
    mods.stroke = Some(ModStroke {
        tool,
        number,
        phase: StrokePhase::Down,
        points: vec![point(at)],
    });
    app.session.gesture = Some(Gesture::ModTool);
}

/// The pointer moved to `at` with the button down.
pub(crate) fn drag_stroke(app: &mut App, at: DVec2) {
    let spacing = SPACING / f64::from(app.session.camera.zoom.max(0.01));
    let Some(stroke) = app.mods.stroke.as_mut() else {
        return;
    };
    let near =
        (stroke.points.last()).is_some_and(|[x, y]| DVec2::new(*x, *y).distance(at) < spacing);
    if !near {
        stroke.points.push(point(at));
    }
}

/// The button came up: the mod sees `up` and commits.
pub(crate) fn finish_stroke(app: &mut App) {
    if let Some(stroke) = app.mods.stroke.as_mut() {
        stroke.phase = StrokePhase::Up;
    }
}

/// The stroke was abandoned (Escape, a tool change): nothing is committed
/// and the preview goes.
pub(crate) fn cancel_stroke(app: &mut App) {
    app.mods.stroke = None;
    app.mods.preview.clear();
}
