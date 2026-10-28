//! Edges: a curve between two entities, or the right-angle route a flow
//! laid out with its corners rounded, arrowheads at its ends and a label at
//! its middle.
//!
//! The path comes from [`App::edge_path`](specular_interact::App::edge_path),
//! the same one hit-testing uses, so an edge is drawn where it is hit. It is
//! in screen space; the line width follows the zoom.

use glam::Vec2;
use specular_doc::{Edge, EdgeEnd, EntityId, ItemId, LineStyle};
use specular_interact::{EdgePath, EdgeRoute};

use super::editing;
use super::frame::{Frame, vec_point};
use super::palette::{self, Palette, Role};
use crate::{
    Color, Dash, Item, LineCap, LineJoin, PathCommand, PathDraw, PathStroke, PolygonDraw, Rect,
    Scene, TextAlign, TextRun, VerticalAlign,
};

const DEFAULT_COLOR: Color = Color::rgb(0x94, 0xa3, 0xb8);
/// Line width in canvas units when the edge sets none.
const DEFAULT_WIDTH: f32 = 1.5;
/// A dashed edge's dash and gap, as multiples of its width.
const DASH_ON: f32 = 3.0;
const DASH_OFF: f32 = 2.0;
/// An arrowhead, in line widths: how far its tip passes the end of the
/// curve, how far back its base sits, and half the base's width.
const ARROW_TIP: f32 = 0.75;
const ARROW_BACK: f32 = 4.25;
const ARROW_HALF_WIDTH: f32 = 3.0;
/// Label size in canvas units.
const LABEL_SIZE: f32 = specular_interact::EDGE_LABEL_SIZE;
/// The radius a route's corners are rounded to, in canvas units, at most:
/// a short run between two corners takes a smaller one.
const CORNER_RADIUS: f32 = 12.0;

pub(crate) fn draw(frame: &Frame<'_>, edge: &Edge, scene: &mut Scene) {
    // The rubber band of a drag stands in for an edge whose end is moving.
    if frame.app.rerouting() == Some(&edge.id) {
        return;
    }
    let Some(path) = frame.app.edge_path(&edge.id) else {
        return;
    };
    let Some(bounds) = Rect::bounding(path.hull().into_iter().map(vec_point)) else {
        return;
    };
    if !frame.sees_screen(bounds) {
        return;
    }
    let selected = frame
        .app
        .session()
        .selection
        .contains(&ItemId::Edge(edge.id.clone()));
    let color = match &edge.color {
        _ if selected && frame.chrome => frame.colors.selection,
        Some(color) => palette::resolve(color, Palette::Vivid, Role::Ink, frame.colors),
        None => DEFAULT_COLOR,
    };
    let width = edge
        .stroke_width
        .map_or(DEFAULT_WIDTH, |width| width as f32)
        * frame.zoom();
    let dash = match edge.line_style.unwrap_or(LineStyle::Solid) {
        LineStyle::Solid => None,
        LineStyle::Dashed => Some(Dash {
            on: width * DASH_ON,
            off: width * DASH_OFF,
        }),
    };
    let commands = match &path {
        EdgePath::Curve(curve) => vec![
            PathCommand::MoveTo(vec_point(curve.from)),
            PathCommand::CubicTo {
                control1: vec_point(curve.from_control),
                control2: vec_point(curve.to_control),
                to: vec_point(curve.to),
            },
        ],
        EdgePath::Route(route) => rounded(route, CORNER_RADIUS * frame.zoom()),
    };
    scene.push(Item::screen(PathDraw {
        commands,
        fill: None,
        stroke: Some(PathStroke {
            color,
            width,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash,
        }),
    }));

    let arrow = |end: Option<EdgeEnd>, default: EdgeEnd| end.unwrap_or(default) == EdgeEnd::Arrow;
    if arrow(edge.from_end, EdgeEnd::None) {
        let heading = heading(path.after_from(), path.from(), path.to());
        scene.push(arrowhead(path.from(), heading, width, color));
    }
    if arrow(edge.to_end, EdgeEnd::Arrow) {
        let heading = heading(path.before_to(), path.to(), path.from());
        scene.push(arrowhead(path.to(), heading, width, color));
    }
    if let Some(shown) = frame.app.editing_edge_label(&edge.id) {
        let key = EntityId::from(edge.id.as_str());
        editing::selection(frame, &key, None, scene);
        scene.extend(editing::edited_line(frame, shown, frame.colors.ink));
        editing::caret(frame, &key, None, frame.colors.ink, scene);
        return;
    }
    match edge.label.as_deref() {
        Some(label) if !label.is_empty() => {
            scene.push(self::label(&path, label, frame.zoom(), frame.colors.ink));
        }
        Some(_) | None => {}
    }
}

/// The direction the curve is travelling as it arrives at `end` from its
/// control point. When the two coincide, the straight line from the other
/// end stands in.
fn heading(control: Vec2, end: Vec2, other_end: Vec2) -> Vec2 {
    let along = (end - control).normalize_or_zero();
    if along == Vec2::ZERO {
        (end - other_end).normalize_or(Vec2::X)
    } else {
        along
    }
}

/// A filled triangle pointing along `heading`, sized by the line `width`.
fn arrowhead(end: Vec2, heading: Vec2, width: f32, color: Color) -> Item {
    let across = heading.perp() * (ARROW_HALF_WIDTH * width);
    let base = end - heading * (ARROW_BACK * width);
    Item::screen(PolygonDraw {
        points: vec![
            vec_point(base + across),
            vec_point(end + heading * (ARROW_TIP * width)),
            vec_point(base - across),
        ],
        fill: Some(color),
        stroke: None,
    })
}

/// The label, centred on the middle of the path and upright.
fn label(path: &EdgePath, text: &str, zoom: f32, color: Color) -> Item {
    Item::screen(TextRun {
        align: TextAlign::Centre,
        vertical_align: VerticalAlign::Middle,
        ..TextRun::new(text, vec_point(path.middle()), LABEL_SIZE * zoom, color)
    })
}

/// A route as straight runs whose corners turn on a quarter curve of up
/// to `radius`, and less where a run is too short to give it either side.
fn rounded(route: &EdgeRoute, radius: f32) -> Vec<PathCommand> {
    let points = &route.points;
    let mut commands = vec![PathCommand::MoveTo(vec_point(points[0]))];
    for corner in 1..points.len() - 1 {
        let (before, at, after) = (points[corner - 1], points[corner], points[corner + 1]);
        let room = (at.distance(before) / 2.0).min(at.distance(after) / 2.0);
        let radius = radius.min(room);
        let into = (at - before).normalize_or_zero();
        let out = (after - at).normalize_or_zero();
        commands.push(PathCommand::LineTo(vec_point(at - into * radius)));
        commands.push(PathCommand::QuadTo {
            control: vec_point(at),
            to: vec_point(at + out * radius),
        });
    }
    commands.push(PathCommand::LineTo(vec_point(points[points.len() - 1])));
    commands
}
