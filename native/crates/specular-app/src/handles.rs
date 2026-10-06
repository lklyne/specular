//! The four resize handles of a selected page: where they are, which one a
//! pointer is on, and the rect a drag of one produces.

use glam::Vec2;
use specular_core::{Camera, CanvasRect};

/// Handle square size in logical pixels; it does not scale with zoom.
pub(crate) const HANDLE_SIZE: f32 = 8.0;
/// Extra logical pixels around a handle that still count as a hit.
const HIT_SLOP: f32 = 4.0;
/// Smallest canvas size a resize may produce.
pub(crate) const MIN_PAGE_SIZE: Vec2 = Vec2::new(120.0, 80.0);

/// A corner of a page rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Corner {
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
}

impl Corner {
    pub(crate) const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomRight,
        Self::BottomLeft,
    ];

    /// `-1` on the axes where this corner sits at the low edge, `1` at the
    /// high edge.
    fn sign(self) -> Vec2 {
        match self {
            Self::TopLeft => Vec2::new(-1.0, -1.0),
            Self::TopRight => Vec2::new(1.0, -1.0),
            Self::BottomRight => Vec2::new(1.0, 1.0),
            Self::BottomLeft => Vec2::new(-1.0, 1.0),
        }
    }

    /// Where this corner of `rect` is, in canvas space.
    pub(crate) fn point(self, rect: CanvasRect) -> Vec2 {
        rect.origin() + rect.size() * (self.sign() * 0.5 + Vec2::splat(0.5))
    }

    /// The corner a resize from this one holds still.
    pub(crate) fn opposite(self) -> Self {
        match self {
            Self::TopLeft => Self::BottomRight,
            Self::TopRight => Self::BottomLeft,
            Self::BottomRight => Self::TopLeft,
            Self::BottomLeft => Self::TopRight,
        }
    }
}

/// The handle of `rect` under `screen`, nearest first when handles overlap
/// on a small page. Hit-tested in screen space so the target size is
/// constant at any zoom.
pub(crate) fn hit(rect: CanvasRect, camera: &Camera, screen: Vec2) -> Option<Corner> {
    let reach = HANDLE_SIZE / 2.0 + HIT_SLOP;
    Corner::ALL
        .into_iter()
        .map(|corner| {
            let offset = (camera.world_to_screen(corner.point(rect)) - screen).abs();
            (corner, offset)
        })
        .filter(|(_, offset)| offset.x <= reach && offset.y <= reach)
        .min_by(|a, b| a.1.length_squared().total_cmp(&b.1.length_squared()))
        .map(|(corner, _)| corner)
}

/// `start` resized by dragging `corner` to `target` (canvas space): the
/// opposite corner stays fixed, and the rect never goes below
/// [`MIN_PAGE_SIZE`] or flips.
pub(crate) fn resized(start: CanvasRect, corner: Corner, target: Vec2) -> CanvasRect {
    let fixed = corner.opposite().point(start);
    let sign = corner.sign();
    let size = ((target - fixed) * sign).max(MIN_PAGE_SIZE);
    let origin = fixed
        + Vec2::new(
            if sign.x > 0.0 { 0.0 } else { -size.x },
            if sign.y > 0.0 { 0.0 } else { -size.y },
        );
    CanvasRect::new(origin.x, origin.y, size.x, size.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: CanvasRect = CanvasRect::new(100.0, 100.0, 400.0, 300.0);

    #[test]
    fn corner_points_are_the_rect_corners() {
        let points = Corner::ALL.map(|corner| corner.point(RECT));
        assert_eq!(
            points,
            [
                Vec2::new(100.0, 100.0),
                Vec2::new(500.0, 100.0),
                Vec2::new(500.0, 400.0),
                Vec2::new(100.0, 400.0),
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
        // A page 6 logical px wide: every handle is within reach.
        let tiny = CanvasRect::new(0.0, 0.0, 6.0, 6.0);
        let camera = Camera::new(Vec2::ZERO, 1.0);
        assert_eq!(
            hit(tiny, &camera, Vec2::new(5.0, 1.0)),
            Some(Corner::TopRight)
        );
    }

    #[test]
    fn dragging_bottom_right_grows_from_the_top_left() {
        let next = resized(RECT, Corner::BottomRight, Vec2::new(600.0, 500.0));
        assert_eq!(next, CanvasRect::new(100.0, 100.0, 500.0, 400.0));
    }

    #[test]
    fn dragging_top_left_holds_the_bottom_right_fixed() {
        let next = resized(RECT, Corner::TopLeft, Vec2::new(0.0, 50.0));
        assert_eq!(next, CanvasRect::new(0.0, 50.0, 500.0, 350.0));
    }

    #[test]
    fn resize_stops_at_the_minimum_size_instead_of_flipping() {
        let next = resized(RECT, Corner::TopLeft, Vec2::new(900.0, 900.0));
        assert_eq!(next, CanvasRect::new(380.0, 320.0, 120.0, 80.0));
    }
}
