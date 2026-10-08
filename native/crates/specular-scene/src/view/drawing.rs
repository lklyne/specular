//! Drawings: each stroke is the filled outline of its pointer path.

use glam::DVec2;
use specular_doc::{BrushType, Drawing, Entity, Stroke};

use super::frame::Frame;
use super::freehand;
use super::palette::{self, Palette, Role};
use crate::cache::StrokeOutline;
use crate::{Blend, Colors, Item, PathCommand, PathDraw, Point, Scene};

/// The outline is this much wider than the stroke's nominal width.
const OUTLINE_SCALE: f64 = 1.6;
/// A stroke never draws thinner than this many logical pixels.
const MIN_SCREEN_WIDTH: f64 = 1.0;
/// How strongly the highlighter's ink is multiplied into what it marks.
/// Multiplied, it tints the paper and leaves dark text as dark as it was,
/// which painting it over at any alpha cannot do.
const HIGHLIGHT_ALPHA: f32 = 0.7;

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, drawing: &Drawing, scene: &mut Scene) {
    let min_width = MIN_SCREEN_WIDTH / f64::from(frame.zoom().max(f32::EPSILON));
    // A stroke is outlined again only when it, or the width it draws at,
    // is not what the frame before drew.
    let mut kept = frame.cache.strokes.take(&entity.id).unwrap_or_default();
    kept.truncate(drawing.strokes.len());
    for (at, stroke) in drawing.strokes.iter().enumerate() {
        let size = stroke.width.max(min_width) * OUTLINE_SCALE;
        let fresh = (kept.get(at))
            .is_some_and(|kept| kept.size.to_bits() == size.to_bits() && kept.stroke == *stroke);
        if !fresh {
            frame.cache.count_built();
            let outline = StrokeOutline {
                stroke: stroke.clone(),
                size,
                item: stroke_item(stroke, size, frame.colors),
            };
            match kept.get_mut(at) {
                Some(slot) => *slot = outline,
                None => kept.push(outline),
            }
        }
        scene.extend(kept[at].item.clone());
    }
    frame.cache.strokes.put(&entity.id, kept);
}

/// The filled outline of `stroke`, `size` units wide.
fn stroke_item(stroke: &Stroke, size: f64, colors: &Colors) -> Option<Item> {
    let brush = stroke.brush.unwrap_or(BrushType::Pen);
    // The pen has round ends. The highlighter's are cut flat, like a marker.
    let (palette, round_ends, alpha, blend) = match brush {
        BrushType::Pen => (Palette::Vivid, true, 1.0, Blend::Normal),
        BrushType::Highlight => (Palette::Soft, false, HIGHLIGHT_ALPHA, colors.highlight),
    };
    let path: Vec<DVec2> = stroke
        .points
        .iter()
        .map(|point| DVec2::new(point.x, point.y))
        .collect();
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
    let ink = palette::resolve(&stroke.color, palette, Role::Ink, colors);
    let path = PathDraw {
        commands,
        fill: Some(palette::with_alpha(ink, alpha)),
        stroke: None,
    };
    Some(Item::canvas(path).with_blend(blend))
}
