//! The shape catalog's geometry: the silhouette of each [`ShapeKind`] in a
//! given rect, and where its label goes.
//!
//! Straight-sided kinds and the cylinder are defined in a 0 to 100 box and
//! stretch with the rect. The rounded kinds keep true circular corners at
//! any aspect ratio.

use specular_doc::ShapeKind;

use crate::{PathCommand, Point, Rect};

/// Corner radius of the rounded rectangle, in canvas units. Resizing
/// lengthens the flat sides and leaves the corners alone until the box is
/// too small for them.
const CORNER_RADIUS: f32 = 24.0;
/// How far along a quarter arc's tangent its cubic control points sit.
const KAPPA: f32 = 0.552_284_8;

/// A shape's outline.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Silhouette {
    /// A rect with circular corners of this radius.
    Rect(f32),
    /// The ellipse inscribed in the rect.
    Ellipse,
    /// Straight sides through these corners.
    Polygon(Vec<Point>),
    /// Curved sides.
    Path(Vec<PathCommand>),
}

/// The outline of `kind` filling `rect`.
pub(crate) fn silhouette(kind: ShapeKind, rect: Rect) -> Silhouette {
    let at = |x: f32, y: f32| normalized(rect, x, y);
    let polygon = |corners: &[(f32, f32)]| {
        Silhouette::Polygon(corners.iter().map(|&(x, y)| at(x, y)).collect())
    };
    match kind {
        ShapeKind::Rectangle => Silhouette::Rect(0.0),
        ShapeKind::Rounded => Silhouette::Rect(CORNER_RADIUS),
        ShapeKind::Pill => Silhouette::Rect(rect.width.min(rect.height) / 2.0),
        ShapeKind::Ellipse => Silhouette::Ellipse,
        ShapeKind::Diamond => polygon(&[(50.0, 0.0), (100.0, 50.0), (50.0, 100.0), (0.0, 50.0)]),
        ShapeKind::Triangle => polygon(&[(50.0, 6.7), (100.0, 93.3), (0.0, 93.3)]),
        ShapeKind::Hexagon => polygon(&[
            (25.0, 6.7),
            (75.0, 6.7),
            (100.0, 50.0),
            (75.0, 93.3),
            (25.0, 93.3),
            (0.0, 50.0),
        ]),
        ShapeKind::Parallelogram => {
            polygon(&[(22.0, 0.0), (100.0, 0.0), (78.0, 100.0), (0.0, 100.0)])
        }
        ShapeKind::Chevron => polygon(&[
            (0.0, 0.0),
            (70.0, 0.0),
            (100.0, 50.0),
            (70.0, 100.0),
            (0.0, 100.0),
            (30.0, 50.0),
        ]),
        ShapeKind::Cylinder => Silhouette::Path(vec![
            PathCommand::MoveTo(at(0.0, 15.0)),
            quarter(at(0.0, 15.0), at(0.0, 0.0), at(50.0, 0.0)),
            quarter(at(50.0, 0.0), at(100.0, 0.0), at(100.0, 15.0)),
            PathCommand::LineTo(at(100.0, 85.0)),
            quarter(at(100.0, 85.0), at(100.0, 100.0), at(50.0, 100.0)),
            quarter(at(50.0, 100.0), at(0.0, 100.0), at(0.0, 85.0)),
            PathCommand::Close,
        ]),
    }
}

/// A stroke-only line drawn over the silhouette: the front of a cylinder's
/// top rim. `None` for every other kind.
pub(crate) fn overlay(kind: ShapeKind, rect: Rect) -> Option<Vec<PathCommand>> {
    let at = |x: f32, y: f32| normalized(rect, x, y);
    match kind {
        ShapeKind::Cylinder => Some(vec![
            PathCommand::MoveTo(at(0.0, 15.0)),
            quarter(at(0.0, 15.0), at(0.0, 30.0), at(50.0, 30.0)),
            quarter(at(50.0, 30.0), at(100.0, 30.0), at(100.0, 15.0)),
        ]),
        ShapeKind::Rectangle
        | ShapeKind::Rounded
        | ShapeKind::Ellipse
        | ShapeKind::Diamond
        | ShapeKind::Triangle
        | ShapeKind::Hexagon
        | ShapeKind::Pill
        | ShapeKind::Parallelogram
        | ShapeKind::Chevron => None,
    }
}

/// The box the label is laid out in: the part of `rect` the silhouette
/// covers well enough to hold text.
pub(crate) fn label_box(kind: ShapeKind, rect: Rect) -> Rect {
    let (x, y, width, height) = match kind {
        ShapeKind::Diamond => (25.0, 25.0, 50.0, 50.0),
        ShapeKind::Triangle => (20.0, 48.0, 60.0, 42.0),
        ShapeKind::Chevron => (5.0, 15.0, 60.0, 70.0),
        ShapeKind::Cylinder => (8.0, 28.0, 84.0, 58.0),
        ShapeKind::Rectangle
        | ShapeKind::Rounded
        | ShapeKind::Ellipse
        | ShapeKind::Hexagon
        | ShapeKind::Pill
        | ShapeKind::Parallelogram => return rect,
    };
    let origin = normalized(rect, x, y);
    Rect::new(
        origin.x,
        origin.y,
        rect.width * width / 100.0,
        rect.height * height / 100.0,
    )
}

impl Silhouette {
    /// The outline as a closed path, for the dashed border the distance-field
    /// shapes cannot draw.
    pub(crate) fn into_path(self, rect: Rect) -> Vec<PathCommand> {
        match self {
            Self::Rect(radius) => rounded_rect(rect, radius),
            Self::Ellipse => {
                let centre = rect.centre();
                let top = Point::new(centre.x, rect.y);
                let right = Point::new(rect.right(), centre.y);
                let bottom = Point::new(centre.x, rect.bottom());
                let left = Point::new(rect.x, centre.y);
                vec![
                    PathCommand::MoveTo(top),
                    quarter(top, Point::new(rect.right(), rect.y), right),
                    quarter(right, Point::new(rect.right(), rect.bottom()), bottom),
                    quarter(bottom, Point::new(rect.x, rect.bottom()), left),
                    quarter(left, rect.origin(), top),
                    PathCommand::Close,
                ]
            }
            Self::Polygon(points) => {
                let mut commands: Vec<PathCommand> = points
                    .into_iter()
                    .enumerate()
                    .map(|(index, point)| {
                        if index == 0 {
                            PathCommand::MoveTo(point)
                        } else {
                            PathCommand::LineTo(point)
                        }
                    })
                    .collect();
                commands.push(PathCommand::Close);
                commands
            }
            Self::Path(commands) => commands,
        }
    }
}

/// A point given in the catalog's 0 to 100 box, placed in `rect`.
fn normalized(rect: Rect, x: f32, y: f32) -> Point {
    Point::new(
        rect.x + rect.width * x / 100.0,
        rect.y + rect.height * y / 100.0,
    )
}

/// A quarter of an axis-aligned ellipse from `from` to `to`, bulging towards
/// `corner`, the corner of the box around the arc.
fn quarter(from: Point, corner: Point, to: Point) -> PathCommand {
    let towards = |point: Point| {
        Point::new(
            point.x + (corner.x - point.x) * KAPPA,
            point.y + (corner.y - point.y) * KAPPA,
        )
    };
    PathCommand::CubicTo {
        control1: towards(from),
        control2: towards(to),
        to,
    }
}

fn rounded_rect(rect: Rect, radius: f32) -> Vec<PathCommand> {
    let r = radius.min(rect.width / 2.0).min(rect.height / 2.0).max(0.0);
    let (left, top, right, bottom) = (rect.x, rect.y, rect.right(), rect.bottom());
    let at = Point::new;
    if r == 0.0 {
        return vec![
            PathCommand::MoveTo(at(left, top)),
            PathCommand::LineTo(at(right, top)),
            PathCommand::LineTo(at(right, bottom)),
            PathCommand::LineTo(at(left, bottom)),
            PathCommand::Close,
        ];
    }
    vec![
        PathCommand::MoveTo(at(left + r, top)),
        PathCommand::LineTo(at(right - r, top)),
        quarter(at(right - r, top), at(right, top), at(right, top + r)),
        PathCommand::LineTo(at(right, bottom - r)),
        quarter(
            at(right, bottom - r),
            at(right, bottom),
            at(right - r, bottom),
        ),
        PathCommand::LineTo(at(left + r, bottom)),
        quarter(at(left + r, bottom), at(left, bottom), at(left, bottom - r)),
        PathCommand::LineTo(at(left, top + r)),
        quarter(at(left, top + r), at(left, top), at(left + r, top)),
        PathCommand::Close,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: Rect = Rect::new(100.0, 200.0, 200.0, 100.0);

    #[test]
    fn straight_sided_kinds_stretch_with_the_rect() {
        assert_eq!(
            silhouette(ShapeKind::Diamond, RECT),
            Silhouette::Polygon(vec![
                Point::new(200.0, 200.0),
                Point::new(300.0, 250.0),
                Point::new(200.0, 300.0),
                Point::new(100.0, 250.0),
            ])
        );
    }

    #[test]
    fn the_label_box_is_inset_where_the_silhouette_is_narrow() {
        assert_eq!(
            (
                label_box(ShapeKind::Diamond, RECT),
                label_box(ShapeKind::Hexagon, RECT),
                label_box(ShapeKind::Triangle, RECT),
                label_box(ShapeKind::Chevron, RECT),
                label_box(ShapeKind::Cylinder, RECT),
            ),
            (
                Rect::new(150.0, 225.0, 100.0, 50.0),
                RECT,
                Rect::new(140.0, 248.0, 120.0, 42.0),
                Rect::new(110.0, 215.0, 120.0, 70.0),
                Rect::new(116.0, 228.0, 168.0, 58.0),
            )
        );
    }

    #[test]
    fn a_rounded_rect_path_clamps_its_radius_to_the_box() {
        let path = Silhouette::Rect(500.0).into_path(Rect::new(0.0, 0.0, 40.0, 20.0));
        // Radius 10: the top side runs from x = 10 to x = 30.
        assert_eq!(
            path[..2],
            [
                PathCommand::MoveTo(Point::new(10.0, 0.0)),
                PathCommand::LineTo(Point::new(30.0, 0.0))
            ]
        );
        // The width limits it as well: in a tall box the sides meet.
        let tall = Silhouette::Rect(500.0).into_path(Rect::new(0.0, 0.0, 20.0, 40.0));
        assert_eq!(
            tall[..2],
            [
                PathCommand::MoveTo(Point::new(10.0, 0.0)),
                PathCommand::LineTo(Point::new(10.0, 0.0))
            ]
        );
        // The corner is a quarter ellipse with the usual handle length.
        let PathCommand::CubicTo {
            control1,
            control2,
            to,
        } = path[2]
        else {
            panic!("a corner is a cubic");
        };
        let handle = 10.0 * 0.552_284_8;
        assert_eq!(
            (control1, control2, to),
            (
                Point::new(30.0 + handle, 0.0),
                Point::new(40.0, 10.0 - handle),
                Point::new(40.0, 10.0)
            )
        );
    }

    #[test]
    fn no_radius_is_a_plain_rectangle_path() {
        assert_eq!(
            Silhouette::Rect(0.0).into_path(Rect::new(0.0, 0.0, 40.0, 20.0)),
            [
                PathCommand::MoveTo(Point::new(0.0, 0.0)),
                PathCommand::LineTo(Point::new(40.0, 0.0)),
                PathCommand::LineTo(Point::new(40.0, 20.0)),
                PathCommand::LineTo(Point::new(0.0, 20.0)),
                PathCommand::Close,
            ]
        );
    }
}
