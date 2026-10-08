//! What edges add to the session layer: the dots on an entity's anchors and
//! the rubber band of an edge being dragged. Everything is in screen space
//! at a fixed pixel size.

use glam::Vec2;

use super::frame::{Frame, vec_point};
use super::palette;
use crate::{
    Color, Dash, EllipseDraw, Item, LineCap, LineJoin, PathCommand, PathDraw, PathStroke, Rect,
    Scene, Stroke, StrokeAlign,
};

/// An anchor's dot: white with a hairline ring.
const DOT_RADIUS: f32 = 4.0;
const DOT_RING: f32 = 1.0;
/// The rubber band: a dashed line with a filled dot where it is pinned and,
/// when it would land on an anchor, a larger ringed dot there.
const BAND_WIDTH: f32 = 2.0;
const BAND_DASH: Dash = Dash { on: 6.0, off: 4.0 };
const ORIGIN_RADIUS: f32 = 3.0;
const SNAP_RADIUS: f32 = 5.0;
const SNAP_RING: f32 = 2.0;

pub(crate) fn draw(frame: &Frame<'_>, scene: &mut Scene) {
    let app = frame.app;
    let on_screen = |anchor: &specular_interact::Anchor| {
        let reach = Vec2::splat(DOT_RADIUS + DOT_RING);
        frame.sees_screen(Rect::new(
            anchor.point.x - reach.x,
            anchor.point.y - reach.y,
            reach.x * 2.0,
            reach.y * 2.0,
        ))
    };
    for anchor in (app.anchors().into_iter()).filter(|anchor| anchor.active && on_screen(anchor)) {
        let ring = Stroke::new(palette::SELECTION, DOT_RING, StrokeAlign::Centre);
        scene.push(dot(anchor.point, DOT_RADIUS, Color::WHITE, Some(ring)));
    }
    let Some(preview) = app.edge_preview() else {
        return;
    };
    let curve = preview.curve;
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
            color: palette::SELECTION,
            width: BAND_WIDTH,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: Some(BAND_DASH),
        }),
    }));
    scene.push(dot(preview.origin, ORIGIN_RADIUS, palette::SELECTION, None));
    if let Some(snap) = preview.snap {
        let ring = Stroke::new(palette::SELECTION, SNAP_RING, StrokeAlign::Centre);
        scene.push(dot(snap, SNAP_RADIUS, Color::WHITE, Some(ring)));
    }
}

fn dot(centre: Vec2, radius: f32, fill: Color, ring: Option<Stroke>) -> Item {
    let rect = Rect::new(
        centre.x - radius,
        centre.y - radius,
        radius * 2.0,
        radius * 2.0,
    );
    let ellipse = EllipseDraw::filled(rect, fill);
    Item::screen(match ring {
        Some(ring) => ellipse.with_stroke(ring),
        None => ellipse,
    })
}
