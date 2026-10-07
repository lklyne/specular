//! Text entities: plain text, and sticky notes.

use specular_doc::{ColorPreset, Entity, Text, TextFont, TextStyle, WidthMode};

use super::frame::canvas_rect;
use super::palette::{self, Palette, Role};
use crate::{FontFamily, Item, Point, RectDraw, Scene, TextRun};

/// Text size when the entity sets none.
const DEFAULT_SIZE: f32 = 14.0;
/// Room kept clear on the right of plain text, so a caret fits.
const PLAIN_RIGHT_PADDING: f32 = 8.0;
/// Space between a sticky note's edge and its text.
const STICKY_PADDING: f32 = 8.0;
/// A sticky note with no colour is yellow.
const STICKY_DEFAULT: specular_doc::Color = specular_doc::Color::Preset(ColorPreset::Yellow);

pub(crate) fn draw(entity: &Entity, text: &Text, scene: &mut Scene) {
    let rect = canvas_rect(entity.rect);
    match text.resolved_style() {
        TextStyle::Plain => {
            let color = palette::resolve_or_neutral(text.color.as_ref(), Palette::Vivid, Role::Ink);
            let wrap_width = match text.resolved_width_mode() {
                WidthMode::Auto => None,
                WidthMode::Fixed => Some((rect.width - PLAIN_RIGHT_PADDING).max(0.0)),
            };
            if let Some(run) = run(text, rect.origin(), wrap_width, color) {
                scene.push(Item::canvas(run));
            }
        }
        TextStyle::Sticky => {
            let stored = text.color.as_ref().unwrap_or(&STICKY_DEFAULT);
            let fill = palette::resolve(stored, Palette::Soft, Role::Fill);
            scene.push(Item::canvas(RectDraw::filled(rect, fill)));
            let inner = rect.outset(-STICKY_PADDING);
            // Clipped, so the renderer can cull a note without shaping it and
            // long text cannot spill over its neighbours.
            if let Some(run) = run(text, inner.origin(), Some(inner.width), palette::INK) {
                scene.push(Item::canvas(run).clipped(rect));
            }
        }
    }
}

/// The entity's text as one run, or `None` when there is nothing to set.
fn run(
    text: &Text,
    origin: Point,
    wrap_width: Option<f32>,
    color: crate::Color,
) -> Option<TextRun> {
    if text.text.is_empty() {
        return None;
    }
    let size = text.size.map_or(DEFAULT_SIZE, |size| size as f32);
    Some(TextRun {
        wrap_width,
        family: family(text.font),
        line_height: size * line_height(size),
        ..TextRun::new(text.text.clone(), origin, size, color)
    })
}

/// Line height as a multiple of the size: roomy for body text, tightening
/// as headings grow.
fn line_height(size: f32) -> f32 {
    (1.5 - (size - 14.0) / 82.0 * 0.4).clamp(1.1, 1.5)
}

fn family(font: Option<TextFont>) -> FontFamily {
    match font {
        Some(TextFont::Sans) | None => FontFamily::SansSerif,
        Some(TextFont::Mono) => FontFamily::Monospace,
        Some(TextFont::Hand) => FontFamily::Named("Kalam".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_height_tightens_as_text_grows() {
        let heights = [14.0, 32.0, 96.0, 144.0].map(|size| (line_height(size) * 1000.0).round());
        assert_eq!(heights, [1500.0, 1412.0, 1100.0, 1100.0]);
    }
}
