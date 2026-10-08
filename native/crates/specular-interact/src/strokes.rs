//! Drawing strokes under a move or a resize. Stroke points are in canvas
//! space, so whatever happens to a drawing's rect has to happen to them.

use glam::DVec2;
use specular_doc::{Point, Rect, Stroke};

/// `strokes` moved by `delta`.
pub(crate) fn translated(strokes: &[Stroke], delta: DVec2) -> Vec<Stroke> {
    mapped(strokes, |point| {
        Point::new(point.x + delta.x, point.y + delta.y)
    })
}

/// `strokes` carried from the rect `from` into the rect `to`. An axis `from`
/// has no extent on keeps its scale.
pub(crate) fn scaled(strokes: &[Stroke], from: Rect, to: Rect) -> Vec<Stroke> {
    let scale = |from: f64, to: f64| if from > 0.0 { to / from } else { 1.0 };
    let (scale_x, scale_y) = (scale(from.width, to.width), scale(from.height, to.height));
    mapped(strokes, |point| {
        Point::new(
            to.x + (point.x - from.x) * scale_x,
            to.y + (point.y - from.y) * scale_y,
        )
    })
}

fn mapped(strokes: &[Stroke], map: impl Fn(Point) -> Point) -> Vec<Stroke> {
    strokes
        .iter()
        .map(|stroke| Stroke {
            points: stroke.points.iter().map(|point| map(*point)).collect(),
            ..stroke.clone()
        })
        .collect()
}

/// The rect a drawing holding `strokes` has: their points, grown by half each
/// stroke's width so a straight line still has a body to grab.
pub(crate) fn bounds(strokes: &[Stroke]) -> Rect {
    let corners = strokes
        .iter()
        .fold(None, |corners: Option<(DVec2, DVec2)>, stroke| {
            let pad = DVec2::splat(stroke.width / 2.0);
            stroke.points.iter().fold(corners, |corners, point| {
                let point = DVec2::new(point.x, point.y);
                let (low, high) = corners.unwrap_or((point - pad, point + pad));
                Some((low.min(point - pad), high.max(point + pad)))
            })
        });
    let Some((low, high)) = corners else {
        return Rect::new(0.0, 0.0, 1.0, 1.0);
    };
    let size = (high - low).max(DVec2::ONE);
    Rect::new(low.x, low.y, size.x, size.y)
}
