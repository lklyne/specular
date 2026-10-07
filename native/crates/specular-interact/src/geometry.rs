//! Bridges between the document's `f64` rects, glam vectors, and the `f32`
//! canvas rects the compositor draws.

use glam::DVec2;
use specular_core::CanvasRect;
use specular_doc::Rect;

/// The rect's top-left corner.
pub(crate) const fn origin(rect: Rect) -> DVec2 {
    DVec2::new(rect.x, rect.y)
}

/// The rect's width and height.
pub(crate) const fn size(rect: Rect) -> DVec2 {
    DVec2::new(rect.width, rect.height)
}

/// The rect at `origin` with `size`.
pub(crate) const fn rect(origin: DVec2, size: DVec2) -> Rect {
    Rect::new(origin.x, origin.y, size.x, size.y)
}

/// The normalised rect with `a` and `b` as opposite corners.
pub(crate) fn spanning(a: DVec2, b: DVec2) -> Rect {
    let low = a.min(b);
    rect(low, a.max(b) - low)
}

/// Whether `point` is inside `rect`: left and top edges count, right and
/// bottom do not.
pub(crate) fn contains(rect: Rect, point: DVec2) -> bool {
    point.x >= rect.x
        && point.y >= rect.y
        && point.x < rect.x + rect.width
        && point.y < rect.y + rect.height
}

/// A document rect as the `f32` rect the compositor and camera work in.
pub fn to_canvas_rect(rect: Rect) -> CanvasRect {
    CanvasRect::new(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
    )
}
