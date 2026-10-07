//! The four resize handles of the selected entity: where they are, which one
//! the pointer is on, and the rect a drag of one produces.

use glam::{DVec2, Vec2};
use specular_core::Camera;
use specular_doc::{EntityId, Kind, Rect};

use crate::{App, geometry};

/// Handle square size in logical pixels. It does not scale with zoom.
pub const HANDLE_SIZE: f32 = 8.0;
/// Extra logical pixels around a handle that still count as a hit.
const HIT_SLOP: f32 = 4.0;
/// Smallest canvas size a page resize may produce.
const MIN_PAGE_SIZE: DVec2 = DVec2::new(120.0, 80.0);

/// A corner of an entity rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Corner {
    /// Low x, low y.
    TopLeft,
    /// High x, low y.
    TopRight,
    /// High x, high y.
    BottomRight,
    /// Low x, high y.
    BottomLeft,
}

impl Corner {
    /// Every corner, clockwise from the top left.
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomRight,
        Self::BottomLeft,
    ];

    /// `-1` on the axes where this corner sits at the low edge, `1` at the
    /// high edge.
    fn sign(self) -> DVec2 {
        match self {
            Self::TopLeft => DVec2::new(-1.0, -1.0),
            Self::TopRight => DVec2::new(1.0, -1.0),
            Self::BottomRight => DVec2::new(1.0, 1.0),
            Self::BottomLeft => DVec2::new(-1.0, 1.0),
        }
    }

    /// Where this corner of `rect` is, in canvas space.
    pub fn point(self, rect: Rect) -> DVec2 {
        geometry::origin(rect) + geometry::size(rect) * (self.sign() * 0.5 + DVec2::splat(0.5))
    }

    /// The corner a resize from this one holds still.
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Self::TopLeft => Self::BottomRight,
            Self::TopRight => Self::BottomLeft,
            Self::BottomRight => Self::TopLeft,
            Self::BottomLeft => Self::TopRight,
        }
    }
}

/// The smallest size a resize may give an entity of this kind, or `None`
/// when the kind has no resize handles yet.
fn min_size(kind: &Kind) -> Option<DVec2> {
    match kind {
        Kind::Page(_) => Some(MIN_PAGE_SIZE),
        Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => None,
    }
}

/// What a handle drag would resize.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResizeTarget<'a> {
    pub(crate) entity: &'a EntityId,
    pub(crate) rect: Rect,
    pub(crate) min_size: DVec2,
}

impl App {
    /// The entity showing resize handles, with its rect: the selection when
    /// it is one resizable entity.
    pub fn handle_target(&self) -> Option<(&EntityId, Rect)> {
        resize_target(self).map(|target| (target.entity, target.rect))
    }
}

pub(crate) fn resize_target(app: &App) -> Option<ResizeTarget<'_>> {
    let entity = app
        .document
        .entity(app.session.selection.single_entity()?)?;
    Some(ResizeTarget {
        entity: &entity.id,
        rect: entity.rect,
        min_size: min_size(&entity.kind)?,
    })
}

/// The handle of `rect` under `screen`, nearest first when handles overlap
/// on a small entity. Hit-tested in screen space so the target size is
/// constant at any zoom.
pub(crate) fn hit(rect: Rect, camera: &Camera, screen: Vec2) -> Option<Corner> {
    let reach = HANDLE_SIZE / 2.0 + HIT_SLOP;
    Corner::ALL
        .into_iter()
        .map(|corner| {
            let at = camera.world_to_screen(corner.point(rect).as_vec2());
            (corner, (at - screen).abs())
        })
        .filter(|(_, offset)| offset.x <= reach && offset.y <= reach)
        .min_by(|a, b| a.1.length_squared().total_cmp(&b.1.length_squared()))
        .map(|(corner, _)| corner)
}

/// `start` resized by dragging `corner` to `target` (canvas space): the
/// opposite corner stays fixed, and the rect never goes below `min_size` or
/// flips.
pub(crate) fn resized(start: Rect, corner: Corner, target: DVec2, min_size: DVec2) -> Rect {
    let fixed = corner.opposite().point(start);
    let sign = corner.sign();
    let size = ((target - fixed) * sign).max(min_size);
    let origin = fixed
        + DVec2::new(
            if sign.x > 0.0 { 0.0 } else { -size.x },
            if sign.y > 0.0 { 0.0 } else { -size.y },
        );
    geometry::rect(origin, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: Rect = Rect::new(100.0, 100.0, 400.0, 300.0);

    #[test]
    fn corner_points_are_the_rect_corners() {
        let points = Corner::ALL.map(|corner| corner.point(RECT));
        assert_eq!(
            points,
            [
                DVec2::new(100.0, 100.0),
                DVec2::new(500.0, 100.0),
                DVec2::new(500.0, 400.0),
                DVec2::new(100.0, 400.0),
            ]
        );
    }

    #[test]
    fn handle_hits_within_slop_of_its_screen_centre() {
        let camera = Camera::new(Vec2::new(10.0, 20.0), 0.5);
        // Bottom-right is at canvas (500, 400) -> screen (260, 220).
        let near = Vec2::new(260.0 + 7.0, 220.0 - 7.0);
        assert_eq!(hit(RECT, &camera, near), Some(Corner::BottomRight));
    }

    #[test]
    fn handle_misses_beyond_slop() {
        let camera = Camera::new(Vec2::ZERO, 1.0);
        assert_eq!(hit(RECT, &camera, Vec2::new(500.0 + 9.0, 400.0)), None);
    }

    #[test]
    fn handle_reach_is_constant_in_screen_pixels_at_any_zoom() {
        let camera = Camera::new(Vec2::ZERO, 0.1);
        let corner = camera.world_to_screen(Vec2::new(500.0, 100.0));
        assert_eq!(
            hit(RECT, &camera, corner + Vec2::new(7.5, 0.0)),
            Some(Corner::TopRight)
        );
    }

    #[test]
    fn overlapping_handles_pick_the_nearest() {
        // An entity 6 logical px wide: every handle is within reach.
        let tiny = Rect::new(0.0, 0.0, 6.0, 6.0);
        let camera = Camera::new(Vec2::ZERO, 1.0);
        assert_eq!(
            hit(tiny, &camera, Vec2::new(5.0, 1.0)),
            Some(Corner::TopRight)
        );
    }

    #[test]
    fn dragging_bottom_right_grows_from_the_top_left() {
        let next = resized(
            RECT,
            Corner::BottomRight,
            DVec2::new(600.0, 500.0),
            MIN_PAGE_SIZE,
        );
        assert_eq!(next, Rect::new(100.0, 100.0, 500.0, 400.0));
    }

    #[test]
    fn dragging_top_left_holds_the_bottom_right_fixed() {
        let next = resized(RECT, Corner::TopLeft, DVec2::new(0.0, 50.0), MIN_PAGE_SIZE);
        assert_eq!(next, Rect::new(0.0, 50.0, 500.0, 350.0));
    }

    #[test]
    fn resize_stops_at_the_minimum_size_instead_of_flipping() {
        let next = resized(
            RECT,
            Corner::TopLeft,
            DVec2::new(900.0, 900.0),
            MIN_PAGE_SIZE,
        );
        assert_eq!(next, Rect::new(380.0, 320.0, 120.0, 80.0));
    }
}
