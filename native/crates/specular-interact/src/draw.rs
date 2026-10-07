//! The draw tool: each press, drag and release is one stroke, and each
//! stroke is its own drawing entity and its own undo step.
//!
//! The drawing is in the document from the press on, growing a point per
//! pointer move, so it is drawn like any other while it is made.

use glam::DVec2;
use specular_doc::{Drawing, Entity, EntityId, JsonMap, Kind, Point, Rect, Stroke};

use crate::{App, Effect, live};

/// A stroke between its press and its release.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawStroke {
    /// The drawing entity the stroke is in.
    id: EntityId,
    stroke: Stroke,
}

impl DrawStroke {
    /// The drawing being made, which is in the document already.
    pub fn drawing(&self) -> &EntityId {
        &self.id
    }

    fn entity(&self) -> Entity {
        let drawing = Drawing {
            strokes: vec![self.stroke.clone()],
        };
        Entity::new(
            self.id.clone(),
            bounds(&self.stroke),
            Kind::Drawing(drawing),
        )
    }
}

/// The rect a drawing holding `stroke` has: its points, grown by half the
/// stroke's width so a straight line still has a body to grab.
fn bounds(stroke: &Stroke) -> Rect {
    let pad = stroke.width / 2.0;
    let corners = stroke.points.iter().fold(None, |corners, point| {
        let point = DVec2::new(point.x, point.y);
        let (low, high) = corners.unwrap_or((point, point));
        Some((low.min(point), high.max(point)))
    });
    let Some((low, high)) = corners else {
        return Rect::new(0.0, 0.0, 1.0, 1.0);
    };
    let size = (high - low + pad * 2.0).max(DVec2::ONE);
    Rect::new(low.x - pad, low.y - pad, size.x, size.y)
}

/// `point` turned about `origin` onto the nearest multiple of 45 degrees,
/// the same distance away.
fn snapped_to_45(origin: Point, point: Point) -> Point {
    let reach = DVec2::new(point.x - origin.x, point.y - origin.y);
    let distance = reach.length();
    if distance == 0.0 {
        return point;
    }
    let step = std::f64::consts::FRAC_PI_4;
    let angle = (reach.y.atan2(reach.x) / step).round() * step;
    Point::new(
        origin.x + angle.cos() * distance,
        origin.y + angle.sin() * distance,
    )
}

/// A press at `world` with the draw tool. Drawing only adds strokes, so the
/// selection is cleared. The brush, color and width are the tool defaults as
/// they are now.
pub(crate) fn begin(app: &mut App, world: DVec2) -> DrawStroke {
    app.session.selection.set([]);
    let defaults = app.tool_defaults.draw.clone();
    let stroke = DrawStroke {
        id: EntityId::from(app.fresh_id().as_str()),
        stroke: Stroke {
            id: app.fresh_id(),
            color: defaults.color,
            width: defaults.stroke_width,
            points: vec![Point::new(world.x, world.y)],
            brush: Some(defaults.brush),
            extra: JsonMap::new(),
        },
    };
    live::put(&mut app.document, stroke.entity());
    stroke
}

/// The pointer moved, or Shift changed, mid-stroke. Shift puts the new point
/// on a 45 degree line from where the stroke began.
pub(crate) fn drag(app: &mut App, stroke: &mut DrawStroke, world: DVec2, shift: bool) {
    let mut point = Point::new(world.x, world.y);
    if shift && let Some(first) = stroke.stroke.points.first() {
        point = snapped_to_45(*first, point);
    }
    if stroke.stroke.points.last() == Some(&point) {
        return;
    }
    stroke.stroke.points.push(point);
    live::put(&mut app.document, stroke.entity());
}

/// The button came up: the drawing becomes one undo step. The tool stays.
pub(crate) fn finish(app: &mut App, stroke: &DrawStroke, effects: &mut Vec<Effect>) {
    live::take(&mut app.document, &stroke.id);
    live::create(app, stroke.entity(), effects);
}

/// The stroke was abandoned.
pub(crate) fn cancel(app: &mut App, stroke: &DrawStroke) {
    live::take(&mut app.document, &stroke.id);
}

#[cfg(test)]
mod tests {
    use specular_doc::Color;

    use super::*;

    fn stroke(width: f64, points: &[(f64, f64)]) -> Stroke {
        Stroke {
            id: "s".to_owned(),
            color: Color::Neutral,
            width,
            points: points.iter().map(|(x, y)| Point::new(*x, *y)).collect(),
            brush: None,
            extra: JsonMap::new(),
        }
    }

    #[test]
    fn bounds_wrap_the_points_and_half_the_width() {
        assert_eq!(
            bounds(&stroke(4.0, &[(10.0, 20.0), (50.0, 5.0), (30.0, 40.0)])),
            Rect::new(8.0, 3.0, 44.0, 39.0)
        );
    }

    #[test]
    fn a_dot_and_a_hairline_still_have_a_body() {
        assert_eq!(
            bounds(&stroke(2.0, &[(10.0, 10.0)])),
            Rect::new(9.0, 9.0, 2.0, 2.0)
        );
        assert_eq!(
            bounds(&stroke(0.0, &[(0.0, 0.0), (10.0, 0.0)])),
            Rect::new(0.0, 0.0, 10.0, 1.0)
        );
        assert_eq!(bounds(&stroke(2.0, &[])), Rect::new(0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn shift_turns_a_point_onto_the_nearest_45_degrees() {
        let origin = Point::new(10.0, 10.0);
        let flat = snapped_to_45(origin, Point::new(110.0, 14.0));
        assert!((flat.y - 10.0).abs() < 1e-9 && flat.x > 110.0);
        let diagonal = snapped_to_45(origin, Point::new(40.0, 45.0));
        assert!((diagonal.x - diagonal.y).abs() < 1e-9);
        assert_eq!(snapped_to_45(origin, origin), origin);
    }
}
