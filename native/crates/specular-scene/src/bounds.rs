//! How much room a draw takes, for culling and for deciding which items
//! overlap.

use crate::{Draw, PathStroke, Rect, Stroke};

impl Draw {
    /// The rect that holds everything this draws, strokes included, in the
    /// item's space.
    ///
    /// `None` when the extent is not known from the geometry alone: text and
    /// columns of it, whose size depends on shaping, and paths and polygons with no points.
    pub fn bounds(&self) -> Option<Rect> {
        match self {
            Self::Page(page) => Some(page.rect),
            Self::Image(image) => Some(image.rect),
            // A blur's tail runs a little past its radius.
            Self::Shadow(shadow) => Some(shadow.rect.outset(shadow.blur.max(0.0) * 1.5)),
            Self::Rect(rect) => Some(rect.rect.outset(shape_outset(rect.stroke))),
            Self::Ellipse(ellipse) => Some(ellipse.rect.outset(shape_outset(ellipse.stroke))),
            Self::Polygon(polygon) => Rect::bounding(polygon.points.iter().copied())
                .map(|rect| rect.outset(path_outset(polygon.stroke))),
            Self::Path(path) => {
                Rect::bounding(path.commands.iter().flat_map(|command| command.points()))
                    .map(|rect| rect.outset(path_outset(path.stroke)))
            }
            Self::Text(_) | Self::Column(_) => None,
        }
    }
}

fn shape_outset(stroke: Option<Stroke>) -> f32 {
    stroke.map_or(0.0, Stroke::outset)
}

fn path_outset(stroke: Option<PathStroke>) -> f32 {
    stroke.map_or(0.0, PathStroke::outset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Color, LineJoin, PathCommand, PathDraw, Point, PolygonDraw, RectDraw, StrokeAlign,
    };

    const RECT: Rect = Rect::new(10.0, 20.0, 100.0, 50.0);

    fn stroked(align: StrokeAlign) -> Draw {
        RectDraw::filled(RECT, Color::WHITE)
            .with_stroke(Stroke::new(Color::BLACK, 4.0, align))
            .into()
    }

    #[test]
    fn centred_and_outside_strokes_grow_a_rect_by_their_reach() {
        assert_eq!(
            [
                stroked(StrokeAlign::Centre).bounds(),
                stroked(StrokeAlign::Outside).bounds(),
                stroked(StrokeAlign::Inside).bounds()
            ],
            [Some(RECT.outset(2.0)), Some(RECT.outset(4.0)), Some(RECT)]
        );
    }

    #[test]
    fn path_bounds_include_control_points_and_stroke_reach() {
        let draw: Draw = PathDraw {
            commands: vec![
                PathCommand::MoveTo(Point::new(0.0, 0.0)),
                PathCommand::CubicTo {
                    control1: Point::new(50.0, -40.0),
                    control2: Point::new(50.0, 40.0),
                    to: Point::new(100.0, 0.0),
                },
            ],
            fill: None,
            stroke: Some(PathStroke::new(Color::BLACK, 4.0)),
        }
        .into();
        assert_eq!(draw.bounds(), Some(Rect::new(-3.0, -43.0, 106.0, 86.0)));
    }

    #[test]
    fn mitered_polygon_reserves_room_for_the_miter() {
        let stroke = PathStroke {
            join: LineJoin::Miter,
            ..PathStroke::new(Color::BLACK, 2.0)
        };
        let draw: Draw = PolygonDraw {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(5.0, 10.0),
            ],
            fill: None,
            stroke: Some(stroke),
        }
        .into();
        assert_eq!(draw.bounds(), Some(Rect::new(-4.0, -4.0, 18.0, 18.0)));
    }
}
