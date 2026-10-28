//! Scene polygons and paths as triangles, tessellated by lyon.
//!
//! Everything is tessellated in logical screen pixels for this frame's
//! camera, so the flattening tolerance and the hairline floor are in pixels
//! and canvas and screen items share one vertex buffer.

use web_time::{Duration, Instant};

use glam::Vec2;
use lyon::math::{Point as LyonPoint, point};
use lyon::path::iterator::PathIterator as _;
use lyon::path::{Path, PathEvent};
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, StrokeOptions,
    StrokeTessellator, StrokeVertex, VertexBuffers,
};
use specular_scene::{Draw, Item, LineCap, LineJoin, PathCommand, PathStroke, Point};

use super::color::linear;
use super::dash::dashes;
use super::place::ViewTransform;
use super::shapes::hairline;
use crate::gpu_types::MeshVertex;

/// Flattening error, in physical pixels.
const TOLERANCE_PX: f32 = 0.2;
/// A dash pattern repeating more often than this along one path is drawn
/// solid: the dashes are too small to see and too many to be worth cutting.
const MAX_DASHES: f32 = 4_096.0;

/// The triangles of every mesh batch of a frame.
pub(crate) type Mesh = VertexBuffers<MeshVertex, u32>;

/// The lyon tessellators, kept for their allocations.
pub(crate) struct Mesher {
    fill: FillTessellator,
    stroke: StrokeTessellator,
    /// Time spent in [`add`](Self::add) since the caller last zeroed it.
    pub(crate) spent: Duration,
}

impl std::fmt::Debug for Mesher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mesher").finish_non_exhaustive()
    }
}

impl Default for Mesher {
    fn default() -> Self {
        Self {
            fill: FillTessellator::new(),
            stroke: StrokeTessellator::new(),
            spent: Duration::ZERO,
        }
    }
}

impl Mesher {
    /// Appends the triangles of a polygon or path item to `mesh`: the fill,
    /// then the stroke over it. Any other draw adds nothing.
    pub(crate) fn add(&mut self, mesh: &mut Mesh, item: &Item, view: &ViewTransform) {
        let started = Instant::now();
        self.tessellate(mesh, item, view);
        self.spent += started.elapsed();
    }

    fn tessellate(&mut self, mesh: &mut Mesh, item: &Item, view: &ViewTransform) {
        let project = |at: Point| {
            let at = view.point(item.space, at);
            point(at.x, at.y)
        };
        let (path, fill, stroke) = match &item.draw {
            Draw::Polygon(polygon) => (
                polygon_path(&polygon.points, project),
                polygon.fill,
                polygon.stroke,
            ),
            Draw::Path(path) => (
                command_path(&path.commands, project),
                path.fill,
                path.stroke,
            ),
            Draw::Page(_)
            | Draw::Shadow(_)
            | Draw::Rect(_)
            | Draw::Ellipse(_)
            | Draw::Text(_)
            | Draw::Column(_)
            | Draw::Image(_) => {
                return;
            }
        };
        let Some(path) = path else {
            return;
        };
        let tolerance = TOLERANCE_PX / view.scale_factor.max(1.0);
        if let Some(fill) = fill {
            let color = linear(fill, item.opacity);
            let result = self.fill.tessellate_path(
                &path,
                // As a 2D canvas fills: a freehand outline crosses itself at
                // its caps and corners, and the overlap is still ink.
                &FillOptions::tolerance(tolerance).with_fill_rule(FillRule::NonZero),
                &mut BuffersBuilder::new(mesh, |vertex: FillVertex<'_>| MeshVertex {
                    position: vertex.position().to_array(),
                    color,
                }),
            );
            if let Err(error) = result {
                tracing::debug!("path fill not tessellated: {error}");
            }
        }
        if let Some(stroke) = stroke.filter(|stroke| stroke.width > 0.0) {
            let scale = view.scale(item.space);
            self.stroke(mesh, &path, stroke, scale, view.scale_factor, item.opacity);
        }
    }

    fn stroke(
        &mut self,
        mesh: &mut Mesh,
        path: &Path,
        stroke: PathStroke,
        scale: f32,
        scale_factor: f32,
        opacity: f32,
    ) {
        let tolerance = TOLERANCE_PX / scale_factor.max(1.0);
        let (width, color) = hairline(stroke.width * scale, stroke.color, scale_factor, opacity);
        let options = StrokeOptions::tolerance(tolerance)
            .with_line_width(width)
            .with_line_cap(match stroke.cap {
                LineCap::Butt => lyon::path::LineCap::Butt,
                LineCap::Round => lyon::path::LineCap::Round,
                LineCap::Square => lyon::path::LineCap::Square,
            })
            .with_line_join(match stroke.join {
                LineJoin::Miter => lyon::path::LineJoin::Miter,
                LineJoin::Round => lyon::path::LineJoin::Round,
                LineJoin::Bevel => lyon::path::LineJoin::Bevel,
            });
        let dashed = stroke
            .dash
            .and_then(|dash| dashed_path(path, dash.on * scale, dash.off * scale, tolerance));
        let result = self.stroke.tessellate_path(
            dashed.as_ref().unwrap_or(path),
            &options,
            &mut BuffersBuilder::new(mesh, |vertex: StrokeVertex<'_, '_>| MeshVertex {
                position: vertex.position().to_array(),
                color,
            }),
        );
        if let Err(error) = result {
            tracing::debug!("path stroke not tessellated: {error}");
        }
    }
}

/// A closed path through `points`; `None` when there are too few or one is
/// not a number.
fn polygon_path(points: &[Point], project: impl Fn(Point) -> LyonPoint) -> Option<Path> {
    if points.len() < 2 || !points.iter().all(|at| finite(*at)) {
        return None;
    }
    let mut builder = Path::builder();
    builder.begin(project(points[0]));
    for &at in &points[1..] {
        builder.line_to(project(at));
    }
    builder.end(true);
    Some(builder.build())
}

/// The lyon path for `commands`; `None` when it is empty or a coordinate is
/// not a number.
fn command_path(commands: &[PathCommand], project: impl Fn(Point) -> LyonPoint) -> Option<Path> {
    let all_finite = commands.iter().all(|command| match *command {
        PathCommand::MoveTo(to) | PathCommand::LineTo(to) => finite(to),
        PathCommand::QuadTo { control, to } => finite(control) && finite(to),
        PathCommand::CubicTo {
            control1,
            control2,
            to,
        } => finite(control1) && finite(control2) && finite(to),
        PathCommand::Close => true,
    });
    if commands.is_empty() || !all_finite {
        return None;
    }
    // The SVG builder takes a segment with no `MoveTo` before it and a path
    // left open at the end, which the plain builder does not.
    let mut builder = Path::builder().with_svg();
    for command in commands {
        match *command {
            PathCommand::MoveTo(to) => {
                builder.move_to(project(to));
            }
            PathCommand::LineTo(to) => {
                builder.line_to(project(to));
            }
            PathCommand::QuadTo { control, to } => {
                builder.quadratic_bezier_to(project(control), project(to));
            }
            PathCommand::CubicTo {
                control1,
                control2,
                to,
            } => {
                builder.cubic_bezier_to(project(control1), project(control2), project(to));
            }
            PathCommand::Close => builder.close(),
        }
    }
    Some(builder.build())
}

fn finite(at: Point) -> bool {
    at.x.is_finite() && at.y.is_finite()
}

/// `path` cut into dashes of `on` and gaps of `off` logical pixels. `None`
/// when the pattern cannot be seen, and the path is then stroked solid.
fn dashed_path(path: &Path, on: f32, off: f32, tolerance: f32) -> Option<Path> {
    if !(on > 0.0 && off > 0.0) {
        return None;
    }
    let mut lines: Vec<Vec<Vec2>> = Vec::new();
    for event in path.iter().flattened(tolerance) {
        match event {
            PathEvent::Begin { at } => lines.push(vec![Vec2::new(at.x, at.y)]),
            PathEvent::Line { to, .. } => {
                if let Some(line) = lines.last_mut() {
                    line.push(Vec2::new(to.x, to.y));
                }
            }
            PathEvent::End { first, close, .. } => {
                if close && let Some(line) = lines.last_mut() {
                    line.push(Vec2::new(first.x, first.y));
                }
            }
            // `flattened` has already turned curves into lines.
            PathEvent::Quadratic { .. } | PathEvent::Cubic { .. } => {}
        }
    }
    let length: f32 = lines
        .iter()
        .flat_map(|line| line.windows(2))
        .map(|pair| pair[0].distance(pair[1]))
        .sum();
    if length / (on + off) > MAX_DASHES {
        return None;
    }
    let mut builder = Path::builder();
    for dash in lines.iter().flat_map(|line| dashes(line, on, off)) {
        let Some((first, rest)) = dash.split_first() else {
            continue;
        };
        builder.begin(point(first.x, first.y));
        for at in rest {
            builder.line_to(point(at.x, at.y));
        }
        builder.end(false);
    }
    Some(builder.build())
}
