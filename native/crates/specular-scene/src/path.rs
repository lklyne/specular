//! Polygons and paths, the geometry the renderer tessellates.

use crate::{Color, Point};

/// How the ends of an open stroke are finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineCap {
    /// Cut square at the end point.
    Butt,
    /// A half circle past the end point.
    #[default]
    Round,
    /// A half square past the end point.
    Square,
}

/// How two stroke segments meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LineJoin {
    /// A sharp corner.
    Miter,
    /// A rounded corner.
    #[default]
    Round,
    /// A flattened corner.
    Bevel,
}

/// A repeating dash pattern, measured along the path in the item's space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dash {
    /// Length of each drawn stretch.
    pub on: f32,
    /// Length of each gap.
    pub off: f32,
}

/// The stroke of a path or polygon, centred on its outline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathStroke {
    /// Stroke colour.
    pub color: Color,
    /// Stroke width, in the item's space.
    pub width: f32,
    /// End caps.
    pub cap: LineCap,
    /// Corner joins.
    pub join: LineJoin,
    /// Dash pattern; `None` is a solid line.
    pub dash: Option<Dash>,
}

impl PathStroke {
    /// A solid stroke with round caps and joins.
    pub const fn new(color: Color, width: f32) -> Self {
        Self {
            color,
            width,
            cap: LineCap::Round,
            join: LineJoin::Round,
            dash: None,
        }
    }

    /// The same stroke, dashed.
    #[must_use]
    pub const fn dashed(self, dash: Dash) -> Self {
        Self {
            dash: Some(dash),
            ..self
        }
    }

    /// How far the stroke can reach past the outline it follows.
    pub fn outset(self) -> f32 {
        let width = self.width.max(0.0);
        // A miter runs up to the usual limit of four half-widths.
        match self.join {
            LineJoin::Miter => width * 2.0,
            LineJoin::Round | LineJoin::Bevel => width * 0.75,
        }
    }
}

/// A closed polygon: shape kinds with straight sides (diamond, triangle,
/// hexagon) and arrowheads.
#[derive(Debug, Clone, PartialEq)]
pub struct PolygonDraw {
    /// The corners in order. The last joins back to the first.
    pub points: Vec<Point>,
    /// Fill colour, if filled.
    pub fill: Option<Color>,
    /// Outline, if stroked.
    pub stroke: Option<PathStroke>,
}

/// One step of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    /// Starts a new sub-path at a point.
    MoveTo(Point),
    /// A straight line to a point.
    LineTo(Point),
    /// A quadratic Bézier.
    QuadTo {
        /// Control point.
        control: Point,
        /// End point.
        to: Point,
    },
    /// A cubic Bézier.
    CubicTo {
        /// First control point.
        control1: Point,
        /// Second control point.
        control2: Point,
        /// End point.
        to: Point,
    },
    /// Closes the current sub-path back to its start.
    Close,
}

impl PathCommand {
    /// Every point the command names, control points included.
    pub(crate) fn points(self) -> impl Iterator<Item = Point> {
        let points = match self {
            Self::MoveTo(to) | Self::LineTo(to) => [Some(to), None, None],
            Self::QuadTo { control, to } => [Some(control), Some(to), None],
            Self::CubicTo {
                control1,
                control2,
                to,
            } => [Some(control1), Some(control2), Some(to)],
            Self::Close => [None; 3],
        };
        points.into_iter().flatten()
    }
}

/// A stroked and/or filled path: freehand strokes, edges, dashed borders and
/// curved shape kinds.
#[derive(Debug, Clone, PartialEq)]
pub struct PathDraw {
    /// The path, starting with a [`PathCommand::MoveTo`].
    pub commands: Vec<PathCommand>,
    /// Fill colour, if filled (non-zero winding).
    pub fill: Option<Color>,
    /// Outline, if stroked.
    pub stroke: Option<PathStroke>,
}

impl PathDraw {
    /// An open, unfilled line through `points`.
    pub fn polyline(points: impl IntoIterator<Item = Point>, stroke: PathStroke) -> Self {
        let commands = points
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
        Self {
            commands,
            fill: None,
            stroke: Some(stroke),
        }
    }
}
