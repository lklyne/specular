//! Where a floating panel goes: beside what it points at, kept inside the
//! viewport.

use glam::Vec2;

use super::metrics::{EDGE_MARGIN, TOOLBAR_HEIGHT};
use super::node::PanelRect;
use crate::Placement;

/// `left` kept `EDGE_MARGIN` inside a viewport `span` wide whose first
/// `covered` pixels a panel of the window lies over. A panel wider than
/// that is pinned to the leading edge and runs off the far one; its content
/// is never squeezed.
fn clamp_left(left: f32, width: f32, span: f32, covered: f32) -> f32 {
    left.min(span - EDGE_MARGIN - width)
        .max(covered + EDGE_MARGIN)
}

/// The top-left corner of a popup of `size` set `gap` from `anchor`, as
/// `popupStyle` in `CanvasItemPopup.tsx` places it: centred on the anchor,
/// kept inside the viewport's sides, and never higher than just under the
/// toolbar, where it stays while any of the anchor shows. `None` once the
/// anchor has left the canvas altogether. `covered` is how much of the
/// viewport's left edge the sidebar lies over, which is not canvas.
pub(super) fn beside(
    anchor: PanelRect,
    placement: Placement,
    gap: f32,
    size: Vec2,
    viewport: Vec2,
    covered: f32,
) -> Option<Vec2> {
    let shows = anchor.x < viewport.x
        && anchor.right() > covered
        && anchor.y < viewport.y
        && anchor.bottom() > TOOLBAR_HEIGHT;
    if !shows {
        return None;
    }
    let left = clamp_left(
        anchor.centre().x - size.x / 2.0,
        size.x,
        viewport.x,
        covered,
    );
    let top = match placement {
        Placement::Above => anchor.y - gap - size.y,
        Placement::Below => anchor.bottom() + gap,
    };
    Some(Vec2::new(left, top.max(TOOLBAR_HEIGHT + EDGE_MARGIN)))
}

/// The top-left corner of a list of `size` hung `offset` under `hang`, the
/// bottom of its trigger. `start` lines its left edge up with the trigger's;
/// otherwise it is centred on it. A list that would run off the bottom of
/// the viewport goes above the trigger when there is room for it there.
pub(super) fn hanging(
    trigger: PanelRect,
    hang: f32,
    offset: f32,
    size: Vec2,
    start: bool,
    viewport: Vec2,
    covered: f32,
) -> Vec2 {
    let left = if start {
        trigger.x
    } else {
        trigger.centre().x - size.x / 2.0
    };
    let below = hang + offset;
    let above = trigger.y - offset - size.y;
    let overflows = below + size.y > viewport.y - EDGE_MARGIN;
    let top = if overflows && above >= EDGE_MARGIN {
        above
    } else {
        below
    };
    Vec2::new(clamp_left(left, size.x, viewport.x, covered), top)
}
