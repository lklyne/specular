//! Bridges between the document's `f64` rects, glam vectors, the `f32`
//! canvas rects the compositor draws, and rects on screen.

use glam::{DVec2, Vec2};
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

/// Whether `point` is inside `rect`: left and top edges count, right and
/// bottom do not.
pub(crate) fn contains(rect: Rect, point: DVec2) -> bool {
    point.x >= rect.x
        && point.y >= rect.y
        && point.x < rect.x + rect.width
        && point.y < rect.y + rect.height
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

/// The smallest rect holding both `a` and `b`.
pub(crate) fn union(a: Rect, b: Rect) -> Rect {
    let low = origin(a).min(origin(b));
    let high = (origin(a) + size(a)).max(origin(b) + size(b));
    rect(low, high - low)
}

/// The part of `a` that `b` covers, or `None` when they share no area.
pub(crate) fn intersection(a: Rect, b: Rect) -> Option<Rect> {
    let low = origin(a).max(origin(b));
    let high = (origin(a) + size(a)).min(origin(b) + size(b));
    (high.x > low.x && high.y > low.y).then(|| rect(low, high - low))
}

/// An axis-aligned rect in logical screen pixels, the space hit-testing
/// works in: handles, anchors and labels keep their size at any zoom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenRect {
    /// The top-left corner.
    pub min: Vec2,
    /// Width and height.
    pub size: Vec2,
}

impl ScreenRect {
    pub(crate) const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            min: Vec2::new(x, y),
            size: Vec2::new(width, height),
        }
    }

    /// The square of side `side` centred on `centre`.
    pub(crate) fn square(centre: Vec2, side: f32) -> Self {
        Self {
            min: centre - Vec2::splat(side / 2.0),
            size: Vec2::splat(side),
        }
    }

    /// `rect` as `camera` shows it.
    pub(crate) fn of(camera: &Camera, rect: Rect) -> Self {
        Self {
            min: camera.world_to_screen(origin(rect).as_vec2()),
            size: size(rect).as_vec2() * camera.zoom,
        }
    }

    /// The bottom-right corner.
    pub fn max(self) -> Vec2 {
        self.min + self.size
    }

    /// The middle.
    pub fn centre(self) -> Vec2 {
        self.min + self.size / 2.0
    }

    /// Whether `point` is inside, all four edges included.
    pub fn contains(self, point: Vec2) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max()).all()
    }

    /// The rect grown by `by` on every side. A negative `by` shrinks it.
    pub(crate) fn inflated(self, by: f32) -> Self {
        Self {
            min: self.min - Vec2::splat(by),
            size: self.size + Vec2::splat(by * 2.0),
        }
    }
}
