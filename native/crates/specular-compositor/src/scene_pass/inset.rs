//! A fill's outline moved inwards, so its feathered band straddles the true
//! edge ([`super::fringe`]). Pure.

use glam::Vec2;
use lyon::math::point;
use lyon::path::iterator::PathIterator as _;
use lyon::path::{Path, PathEvent};

use super::fringe::MITER_LIMIT;

/// `path` flattened, closed and moved `by` inwards along each corner's
/// miter. Inwards is taken from the whole path's winding, so a hole wound
/// the other way grows. A part narrower than twice `by` turns inside out,
/// and a non-zero fill still fills it: a fill thinner than a device pixel
/// is drawn a little heavier than it is.
pub(crate) fn inset(path: &Path, by: f32, tolerance: f32) -> Path {
    let rings = rings(path, tolerance);
    let area: f32 = rings.iter().map(|ring| twice_area(ring)).sum();
    // An edge's left-hand normal points inside a ring of positive area.
    let inwards = if area >= 0.0 { 1.0 } else { -1.0 };
    let mut builder = Path::builder();
    for ring in rings.iter().filter(|ring| ring.len() >= 3) {
        let count = ring.len();
        let normal = |at: usize| {
            (ring[(at + 1) % count] - ring[at])
                .perp()
                .normalize_or_zero()
                * inwards
        };
        for (at, &corner) in ring.iter().enumerate() {
            let moved = corner + miter(normal((at + count - 1) % count), normal(at)) * by;
            if at == 0 {
                builder.begin(point(moved.x, moved.y));
            } else {
                builder.line_to(point(moved.x, moved.y));
            }
        }
        builder.end(true);
    }
    builder.build()
}

/// Each sub-path of `path` as a polyline, with no point repeated next to
/// itself and none for the closing edge.
fn rings(path: &Path, tolerance: f32) -> Vec<Vec<Vec2>> {
    let mut rings: Vec<Vec<Vec2>> = Vec::new();
    let add = |rings: &mut Vec<Vec<Vec2>>, at: Vec2| {
        if let Some(ring) = rings.last_mut()
            && ring
                .last()
                .is_none_or(|last| last.distance(at) > f32::EPSILON)
        {
            ring.push(at);
        }
    };
    for event in path.iter().flattened(tolerance) {
        match event {
            PathEvent::Begin { at } => rings.push(vec![Vec2::new(at.x, at.y)]),
            PathEvent::Line { to, .. } => add(&mut rings, Vec2::new(to.x, to.y)),
            PathEvent::End { .. } => {
                if let Some(ring) = rings.last_mut()
                    && ring.len() > 1
                    && ring[0].distance(ring[ring.len() - 1]) <= f32::EPSILON
                {
                    ring.pop();
                }
            }
            // `flattened` has already turned curves into lines.
            PathEvent::Quadratic { .. } | PathEvent::Cubic { .. } => {}
        }
    }
    rings
}

fn twice_area(ring: &[Vec2]) -> f32 {
    let count = ring.len();
    (0..count)
        .map(|at| ring[at].perp_dot(ring[(at + 1) % count]))
        .sum()
}

/// The move that shifts both edges with unit normals `before` and `after`
/// by one unit, held to [`MITER_LIMIT`] at a sharp corner.
fn miter(before: Vec2, after: Vec2) -> Vec2 {
    let Some(direction) = (before + after).try_normalize() else {
        return Vec2::ZERO;
    };
    direction / direction.dot(after).max(1.0 / MITER_LIMIT)
}
