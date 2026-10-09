//! Where a floating list goes: under the row of the chrome its trigger is
//! in, kept inside the viewport's sides.

use glam::Vec2;

use super::metrics::EDGE_MARGIN;
use super::node::PanelRect;

/// The top-left corner of a list `width` wide hung `offset` under `hang`,
/// the bottom of the row its trigger is in. `start` lines its left edge up
/// with the trigger's; otherwise it is centred on it. A list wider than the
/// viewport is pinned to the leading edge and runs off the far one; its
/// content is never squeezed.
pub(super) fn hanging(
    trigger: PanelRect,
    hang: f32,
    offset: f32,
    width: f32,
    start: bool,
    span: f32,
) -> Vec2 {
    let left = if start {
        trigger.x
    } else {
        trigger.centre().x - width / 2.0
    };
    let left = left.min(span - EDGE_MARGIN - width).max(EDGE_MARGIN);
    Vec2::new(left, hang + offset)
}
