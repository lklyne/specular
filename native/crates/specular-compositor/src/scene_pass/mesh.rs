//! Scene polygons and paths as triangles, tessellated by lyon.
//!
//! Everything is tessellated in logical screen pixels for this frame's
//! camera, so the flattening tolerance and the hairline floor are in pixels
//! and canvas and screen items share one vertex buffer.

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
        }
    }
}

impl Mesher {
    /// Appends the triangles of a polygon or path item to `mesh`: the fill,
    /// then the stroke over it. Any other draw adds nothing.
    pub(crate) fn add(&mut self, mesh: &mut Mesh, item: &Item, view: &ViewTransform) {
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

#[cfg(test)]
mod tests {
    use specular_scene::{Color, Dash, PathDraw, PolygonDraw};

    use super::super::place::tests::view;
    use super::*;

    const RED: Color = Color::rgb(255, 0, 0);

    fn mesh_of(item: &Item, zoom: f32) -> Mesh {
        let mut mesh = Mesh::new();
        Mesher::default().add(&mut mesh, item, &view(Vec2::new(10.0, 0.0), zoom));
        mesh
    }

    fn triangle() -> PolygonDraw {
        PolygonDraw {
            points: vec![
                Point::new(0.0, 0.0),
                Point::new(100.0, 0.0),
                Point::new(0.0, 100.0),
            ],
            fill: Some(RED),
            stroke: None,
        }
    }

    fn line(stroke: PathStroke) -> PathDraw {
        PathDraw::polyline([Point::new(0.0, 50.0), Point::new(100.0, 50.0)], stroke)
    }

    fn extent(mesh: &Mesh) -> (Vec2, Vec2) {
        let positions = mesh.vertices.iter().map(|v| Vec2::from(v.position));
        let min = positions.clone().fold(Vec2::MAX, Vec2::min);
        (min, positions.fold(Vec2::MIN, Vec2::max))
    }

    #[test]
    fn a_filled_triangle_is_one_triangle_in_screen_pixels() {
        let mesh = mesh_of(&Item::canvas(triangle()), 2.0);
        assert_eq!(
            (mesh.indices.len(), extent(&mesh)),
            (3, (Vec2::new(10.0, 0.0), Vec2::new(210.0, 200.0)))
        );
    }

    #[test]
    fn where_an_outline_overlaps_itself_it_is_still_filled() {
        let square = [(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)];
        let twice_round = PolygonDraw {
            points: square
                .iter()
                .chain(&square)
                .map(|&(x, y)| Point::new(x, y))
                .collect(),
            fill: Some(RED),
            stroke: None,
        };
        let mesh = mesh_of(&Item::canvas(twice_round), 1.0);
        assert_eq!(
            extent(&mesh),
            (Vec2::new(10.0, 0.0), Vec2::new(110.0, 100.0))
        );
    }

    #[test]
    fn a_screen_polygon_ignores_the_camera() {
        let mesh = mesh_of(&Item::screen(triangle()), 2.0);
        assert_eq!(extent(&mesh), (Vec2::ZERO, Vec2::splat(100.0)));
    }

    #[test]
    fn a_canvas_stroke_widens_with_zoom() {
        let stroke = PathStroke {
            cap: LineCap::Butt,
            ..PathStroke::new(RED, 4.0)
        };
        let (min, max) = extent(&mesh_of(&Item::canvas(line(stroke)), 2.0));
        // 4 units at zoom 2 is 8 px, centred on y = 100.
        assert_eq!((min.y, max.y), (96.0, 104.0));
    }

    #[test]
    fn a_dashed_line_has_gaps() {
        let stroke = PathStroke {
            cap: LineCap::Butt,
            ..PathStroke::new(RED, 2.0).dashed(Dash {
                on: 10.0,
                off: 10.0,
            })
        };
        let mesh = mesh_of(&Item::canvas(line(stroke)), 1.0);
        // No vertex lies inside the first gap, x in (20, 30) with the pan.
        let in_gap = mesh
            .vertices
            .iter()
            .any(|v| v.position[0] > 20.5 && v.position[0] < 29.5);
        assert!(!mesh.indices.is_empty() && !in_gap);
    }

    #[test]
    fn item_opacity_reaches_the_vertices() {
        let mesh = mesh_of(&Item::canvas(triangle()).with_opacity(0.25), 1.0);
        assert!(
            mesh.vertices
                .iter()
                .all(|v| (v.color[3] - 0.25).abs() < 1e-6)
        );
    }

    #[test]
    fn a_path_with_a_bad_coordinate_adds_nothing() {
        let stroke = PathStroke::new(RED, 2.0);
        let bad = PathDraw::polyline([Point::new(0.0, 0.0), Point::new(f32::NAN, 5.0)], stroke);
        assert_eq!(mesh_of(&Item::canvas(bad), 1.0).indices, [0_u32; 0]);
    }

    #[test]
    fn a_path_may_start_without_a_move_and_end_unclosed() {
        let path = PathDraw {
            commands: vec![
                PathCommand::LineTo(Point::new(0.0, 0.0)),
                PathCommand::QuadTo {
                    control: Point::new(50.0, 80.0),
                    to: Point::new(100.0, 0.0),
                },
            ],
            fill: None,
            stroke: Some(PathStroke::new(RED, 2.0)),
        };
        assert_ne!(mesh_of(&Item::canvas(path), 1.0).indices, [0_u32; 0]);
    }
}
