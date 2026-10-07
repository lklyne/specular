//! Edges: a curve between two entities, arrowheads at its ends and a label
//! at its middle.
//!
//! The curve comes from [`App::edge_curve`](specular_interact::App::edge_curve),
//! the same one hit-testing uses, so an edge is drawn where it is hit. It is
//! in screen space; the line width follows the zoom.

use glam::Vec2;
use specular_doc::{Edge, EdgeEnd, ItemId, LineStyle};
use specular_interact::EdgeCurve;

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
const LABEL_SIZE: f32 = 16.0;

pub(crate) fn draw(frame: &Frame<'_>, edge: &Edge, scene: &mut Scene) {
    let Some(curve) = frame.app.edge_curve(&edge.id) else {
        return;
    };
    let ends = [curve.from, curve.from_control, curve.to_control, curve.to];
    let Some(bounds) = Rect::bounding(ends.map(vec_point)) else {
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
        _ if selected && frame.chrome => palette::SELECTION,
        Some(color) => palette::resolve(color, Palette::Vivid, Role::Ink),
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
    scene.push(Item::screen(PathDraw {
        commands: vec![
            PathCommand::MoveTo(vec_point(curve.from)),
            PathCommand::CubicTo {
                control1: vec_point(curve.from_control),
                control2: vec_point(curve.to_control),
                to: vec_point(curve.to),
            },
        ],
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
        let heading = heading(curve.from_control, curve.from, curve.to);
        scene.push(arrowhead(curve.from, heading, width, color));
    }
    if arrow(edge.to_end, EdgeEnd::Arrow) {
        let heading = heading(curve.to_control, curve.to, curve.from);
        scene.push(arrowhead(curve.to, heading, width, color));
    }
    match edge.label.as_deref() {
        Some(label) if !label.is_empty() => scene.push(self::label(&curve, label, frame.zoom())),
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

/// The label, centred on the middle of the curve and upright.
fn label(curve: &EdgeCurve, text: &str, zoom: f32) -> Item {
    Item::screen(TextRun {
        align: TextAlign::Centre,
        vertical_align: VerticalAlign::Middle,
        ..TextRun::new(
            text,
            vec_point(curve.point(0.5)),
            LABEL_SIZE * zoom,
            palette::INK,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_follows_the_control_point_and_falls_back_to_the_chord() {
        let end = Vec2::new(10.0, 0.0);
        assert_eq!(
            (
                heading(Vec2::new(10.0, -5.0), end, Vec2::ZERO),
                heading(end, end, Vec2::ZERO)
            ),
            (Vec2::Y, Vec2::X)
        );
    }

    #[test]
    fn an_arrowhead_points_along_its_heading() {
        let item = arrowhead(Vec2::new(100.0, 50.0), Vec2::X, 2.0, DEFAULT_COLOR);
        let crate::Draw::Polygon(polygon) = item.draw else {
            panic!("an arrowhead is a polygon");
        };
        assert_eq!(
            polygon.points,
            [
                crate::Point::new(91.5, 56.0),
                crate::Point::new(101.5, 50.0),
                crate::Point::new(91.5, 44.0),
            ]
        );
    }
}
