//! Bridges between the document's `f64` rects, glam vectors, the `f32`
//! canvas rects the compositor draws, and rects on screen.

use glam::DVec2;
use specular_core::Camera;
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

/// A document rect as the `f32` rect the compositor and camera work in.
pub fn to_canvas_rect(rect: Rect) -> specular_core::Rect {
    specular_core::Rect::new(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
    )
}

/// A rect in logical screen pixels, the space hit-testing works in:
/// handles, anchors and labels keep their size at any zoom.
pub type ScreenRect = specular_core::Rect;

/// `rect` as `camera` shows it.
pub(crate) fn on_screen(camera: &Camera, rect: Rect) -> ScreenRect {
    ScreenRect::at(
        camera.world_to_screen(origin(rect).as_vec2()),
        size(rect).as_vec2() * camera.zoom,
    )
}
