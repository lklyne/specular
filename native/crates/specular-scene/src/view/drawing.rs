//! Drawings: each stroke is the filled outline of its pointer path.

use glam::DVec2;
use specular_doc::{BrushType, Drawing, Entity, Stroke};

use super::frame::Frame;
use super::freehand;
use super::palette::{self, Palette, Role};
use crate::{Item, PathCommand, PathDraw, Point, Scene};

/// The outline is this much wider than the stroke's nominal width.
const OUTLINE_SCALE: f64 = 1.6;
/// A stroke never draws thinner than this many logical pixels.
const MIN_SCREEN_WIDTH: f64 = 1.0;
/// The highlighter's ink is see-through, so what it marks stays readable.
const HIGHLIGHT_ALPHA: f32 = 0.3;

pub(crate) fn draw(frame: &Frame<'_>, _entity: &Entity, drawing: &Drawing, scene: &mut Scene) {
    let min_width = MIN_SCREEN_WIDTH / f64::from(frame.zoom().max(f32::EPSILON));
    scene.extend(
        drawing
            .strokes
            .iter()
            .filter_map(|stroke| stroke_item(stroke, min_width)),
    );
}

fn stroke_item(stroke: &Stroke, min_width: f64) -> Option<Item> {
    let brush = stroke.brush.unwrap_or(BrushType::Pen);
    // The pen has round ends. The highlighter's are cut flat, like a marker.
    let (palette, round_ends, alpha) = match brush {
        BrushType::Pen => (Palette::Vivid, true, 1.0),
        BrushType::Highlight => (Palette::Soft, false, HIGHLIGHT_ALPHA),
    };
    let path: Vec<DVec2> = stroke
        .points
        .iter()
        .map(|point| DVec2::new(point.x, point.y))
        .collect();
    let size = stroke.width.max(min_width) * OUTLINE_SCALE;
    let outline = freehand::outline(&path, size, round_ends);
    if outline.len() < 3 {
        return None;
    }
    let mut commands: Vec<PathCommand> = outline
        .iter()
        .enumerate()
        .map(|(index, point)| {
            let point = Point::new(point.x as f32, point.y as f32);
            if index == 0 {
                PathCommand::MoveTo(point)
            } else {
                PathCommand::LineTo(point)
            }
        })
        .collect();
    commands.push(PathCommand::Close);
    let ink = palette::resolve(&stroke.color, palette, Role::Ink);
    Some(Item::canvas(PathDraw {
        commands,
        fill: Some(palette::with_alpha(ink, alpha)),
        stroke: None,
    }))
}
