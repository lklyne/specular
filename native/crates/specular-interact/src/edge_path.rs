//! Where an edge runs on screen: the curve between its two entities, and
//! whether a point is on it.

use glam::Vec2;
use specular_doc::{Edge, EdgeId, EdgeSide};

use crate::App;
use crate::geometry::ScreenRect;

/// How far outside its entity an edge starts and ends, in logical pixels.
/// An anchor's dot sits here too.
pub(crate) const ANCHOR_OFFSET: f32 = 8.0;
/// The control points sit between these distances from their ends, in canvas
/// units, so a short edge still leaves its entity square-on and a long one
/// does not balloon.
const CONTROL_MIN: f32 = 40.0;
const CONTROL_MAX: f32 = 200.0;
/// Width of the band along an edge where a press selects it, in logical
/// pixels at zoom 1.
const HIT_WIDTH: f32 = 14.0;
/// Straight pieces an edge is cut into for the distance test.
const SEGMENTS: u8 = 32;

/// The scale of zoom-following hit targets (edge bands, anchor boxes): they
/// shrink with the canvas when zoomed out, but never below this much of
/// their size, and never grow past it when zoomed in.
pub(crate) fn hit_scale(zoom: f32) -> f32 {
    zoom.clamp(0.35, 1.0)
}

/// An edge's cubic bezier in logical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeCurve {
    /// Where the edge leaves its first entity.
    pub from: Vec2,
    /// The control point for `from`.
    pub from_control: Vec2,
    /// The control point for `to`.
    pub to_control: Vec2,
    /// Where the edge reaches its second entity.
    pub to: Vec2,
}

impl EdgeCurve {
    fn between(from: ScreenRect, to: ScreenRect, edge: &Edge, zoom: f32) -> Self {
        let (from_side, to_side) = match (edge.from_side, edge.to_side) {
            (Some(from_side), Some(to_side)) => (from_side, to_side),
            (None, _) | (_, None) => facing_sides(from, to),
        };
        Self::joining(
            (anchor_point(from, from_side), from_side),
            (anchor_point(to, to_side), to_side),
            zoom,
        )
    }

    /// The curve between two points, each leaving in the direction of the
    /// side it is on.
    pub(crate) fn joining(from: (Vec2, EdgeSide), to: (Vec2, EdgeSide), zoom: f32) -> Self {
        let ((start, from_side), (end, to_side)) = (from, to);
        let reach = (start.distance(end) * 0.4).clamp(CONTROL_MIN * zoom, CONTROL_MAX * zoom);
        Self {
            from: start,
            from_control: start + outward(from_side) * reach,
            to_control: end + outward(to_side) * reach,
            to: end,
        }
    }

    /// The point a fraction `t` of the way along, `0.0` to `1.0`.
    pub fn point(&self, t: f32) -> Vec2 {
        let u = 1.0 - t;
        self.from * (u * u * u)
            + self.from_control * (3.0 * u * u * t)
            + self.to_control * (3.0 * u * t * t)
            + self.to * (t * t * t)
    }

    /// The distance from `point` to the nearest part of the curve.
    fn distance_to(&self, point: Vec2) -> f32 {
        let step = |index: u8| self.point(f32::from(index) / f32::from(SEGMENTS));
        (0..SEGMENTS)
            .map(|index| distance_to_segment(point, step(index), step(index + 1)))
            .fold(f32::INFINITY, f32::min)
    }

    /// Whether a press at `screen` is on the edge.
    pub(crate) fn hit(&self, screen: Vec2, zoom: f32) -> bool {
        self.distance_to(screen) <= HIT_WIDTH * hit_scale(zoom) / 2.0
    }
}

impl App {
    /// The curve of the edge `id` on screen, between its entities where
    /// they are seen. `None` when the edge or either entity it names is
    /// missing, or has scrolled out of its page.
    pub fn edge_curve(&self, id: &EdgeId) -> Option<EdgeCurve> {
        let edge = self.document.edge(id)?;
        let camera = &self.session.camera;
        let rect = |entity| self.shown_on_screen(self.document.entity(entity)?);
        Some(EdgeCurve::between(
            rect(&edge.from)?,
            rect(&edge.to)?,
            edge,
            camera.zoom,
        ))
    }
}

impl App {
    /// `entity` on screen where it is seen, which for one that follows its
    /// page is not where it is stored. `None` when it is hidden.
    pub(crate) fn shown_on_screen(&self, entity: &specular_doc::Entity) -> Option<ScreenRect> {
        let rect = crate::shown_rect(self, entity)?;
        Some(ScreenRect::of(&self.session.camera, rect))
    }
}

/// The middle of `side` of `rect`.
pub(crate) fn side_point(rect: ScreenRect, side: EdgeSide) -> Vec2 {
    let (centre, max) = (rect.centre(), rect.max());
    match side {
        EdgeSide::Top => Vec2::new(centre.x, rect.min.y),
        EdgeSide::Bottom => Vec2::new(centre.x, max.y),
        EdgeSide::Left => Vec2::new(rect.min.x, centre.y),
        EdgeSide::Right => Vec2::new(max.x, centre.y),
    }
}

/// Where an edge meets `side` of `rect`, and where that side's anchor dot
/// is: [`ANCHOR_OFFSET`] outside the middle of the side.
pub(crate) fn anchor_point(rect: ScreenRect, side: EdgeSide) -> Vec2 {
    side_point(rect, side) + outward(side) * ANCHOR_OFFSET
}

/// The unit vector pointing away from an entity through `side`.
pub(crate) const fn outward(side: EdgeSide) -> Vec2 {
    match side {
        EdgeSide::Top => Vec2::NEG_Y,
        EdgeSide::Bottom => Vec2::Y,
        EdgeSide::Left => Vec2::NEG_X,
        EdgeSide::Right => Vec2::X,
    }
}

/// The sides two entities face each other with, for an edge that names none.
pub(crate) fn facing_sides(from: ScreenRect, to: ScreenRect) -> (EdgeSide, EdgeSide) {
    let delta = to.centre() - from.centre();
    if delta.x.abs() > delta.y.abs() {
        if delta.x > 0.0 {
            (EdgeSide::Right, EdgeSide::Left)
        } else {
            (EdgeSide::Left, EdgeSide::Right)
        }
    } else if delta.y > 0.0 {
        (EdgeSide::Bottom, EdgeSide::Top)
    } else {
        (EdgeSide::Top, EdgeSide::Bottom)
    }
}

/// The distance from `point` to the nearest part of the segment `a` to `b`.
pub(crate) fn distance_to_segment(point: Vec2, a: Vec2, b: Vec2) -> f32 {
    let along = b - a;
    let length_squared = along.length_squared();
    if length_squared == 0.0 {
        return point.distance(a);
    }
    let t = ((point - a).dot(along) / length_squared).clamp(0.0, 1.0);
    point.distance(a + along * t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_edge_with_no_sides_leaves_from_the_facing_ones() {
        let left = ScreenRect::new(0.0, 0.0, 100.0, 100.0);
        let right = ScreenRect::new(300.0, 20.0, 100.0, 100.0);
        let curve = EdgeCurve::between(left, right, &Edge::new("e", "a", "b"), 1.0);
        assert_eq!(
            (curve.from, curve.to),
            (Vec2::new(108.0, 50.0), Vec2::new(292.0, 70.0))
        );
    }

    #[test]
    fn control_points_reach_outward_within_their_limits() {
        let left = ScreenRect::new(0.0, 0.0, 100.0, 100.0);
        let near = ScreenRect::new(120.0, 0.0, 100.0, 100.0);
        let curve = EdgeCurve::between(left, near, &Edge::new("e", "a", "b"), 1.0);
        // The ends are 4 px apart, so the 40 px minimum applies.
        assert_eq!(
            (curve.from_control.x, curve.to_control.x),
            (108.0 + 40.0, 112.0 - 40.0)
        );
    }
}
