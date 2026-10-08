//! The inspect tool's outline and popover, over everything the canvas draws.
//!
//! The outline is `createHighlight` in `dom-inspection.ts`: a dashed blue
//! border and a faint blue wash. The card is `InspectPopoverLayer.tsx`: the
//! tag on a blue chip, the size after the id and classes, the font, and the
//! text and background colours as squares with their values.

use specular_interact::{InspectModel, InspectPopover};

use super::frame::Frame;
use super::palette::{self, SELECTION};
use crate::{
    Color, Dash, FontFamily, Item, PathDraw, PathStroke, Point, Rect, RectDraw, Scene, ShadowDraw,
    Stroke, StrokeAlign, TextRun, VerticalAlign,
};

const OUTLINE_STROKE_ALPHA: f32 = 0.95;
const OUTLINE_FILL_ALPHA: f32 = 0.14;
const OUTLINE_DASH: Dash = Dash { on: 3.0, off: 3.0 };

const CARD: Color = Color::rgba(30, 41, 59, 242);
const CARD_RADIUS: f32 = 6.0;
const CARD_SHADOW: Color = Color::rgba(0, 0, 0, 64);
const CARD_SHADOW_DROP: f32 = 2.0;
const CARD_SHADOW_BLUR: f32 = 8.0;
const CHIP_RADIUS: f32 = 4.0;
const SWATCH_RADIUS: f32 = 2.0;
const SWATCH_EDGE: Color = Color::rgba(255, 255, 255, 89);
const SOFT: u8 = 230;
const FAINT: u8 = 180;
const LABEL: u8 = 217;

/// The outline on the node, then the card beside it.
pub(crate) fn draw(frame: &Frame<'_>, scene: &mut Scene) {
    let Some(InspectModel { outline, popover }) = frame.app.inspect() else {
        return;
    };
    let target = Rect::new(outline.min.x, outline.min.y, outline.size.x, outline.size.y);
    if frame.sees_screen(target) {
        scene.push(Item::screen(RectDraw::filled(
            target,
            palette::with_alpha(SELECTION, OUTLINE_FILL_ALPHA),
        )));
        scene.push(Item::screen(PathDraw {
            commands: super::shape_path::Silhouette::Rect(0.0).into_path(target.outset(-0.5)),
            fill: None,
            stroke: Some(
                PathStroke::new(palette::with_alpha(SELECTION, OUTLINE_STROKE_ALPHA), 1.0)
                    .dashed(OUTLINE_DASH),
            ),
        }));
    }
    card(&popover, scene);
}

fn card(popover: &InspectPopover, scene: &mut Scene) {
    let min = popover.rect.min;
    let rect = Rect::new(min.x, min.y, popover.rect.size.x, popover.rect.size.y);
    scene.push(Item::screen(ShadowDraw {
        rect: Rect::new(rect.x, rect.y + CARD_SHADOW_DROP, rect.width, rect.height),
        corner_radius: CARD_RADIUS,
        blur: CARD_SHADOW_BLUR,
        color: CARD_SHADOW,
    }));
    scene.push(Item::screen(
        RectDraw::filled(rect, CARD).with_corner_radius(CARD_RADIUS),
    ));
    let text = |text: &str, at: Point, height: f32, alpha: u8| {
        Item::screen(TextRun {
            family: FontFamily::Monospace,
            line_height: InspectPopover::LINE,
            box_height: Some(height),
            vertical_align: VerticalAlign::Middle,
            ..TextRun::new(
                text,
                at,
                InspectPopover::FONT_SIZE,
                Color::WHITE.with_alpha(alpha),
            )
        })
        .clipped(rect)
    };
    let left = rect.x + InspectPopover::PADDING;
    let mut top = rect.y + InspectPopover::PADDING;

    let chip_width = InspectPopover::text_width(&popover.tag) + 2.0 * InspectPopover::CHIP_PADDING;
    scene.push(Item::screen(
        RectDraw::filled(
            Rect::new(left, top, chip_width, InspectPopover::CHIP_HEIGHT),
            SELECTION,
        )
        .with_corner_radius(CHIP_RADIUS),
    ));
    scene.push(text(
        &popover.tag,
        Point::new(left + InspectPopover::CHIP_PADDING, top),
        InspectPopover::CHIP_HEIGHT,
        255,
    ));
    let mut x = left + chip_width;
    for (part, alpha) in [(&popover.remainder, SOFT), (&popover.size, FAINT)] {
        if part.is_empty() {
            continue;
        }
        x += InspectPopover::GAP;
        scene.push(text(
            part,
            Point::new(x, top),
            InspectPopover::CHIP_HEIGHT,
            alpha,
        ));
        x += InspectPopover::text_width(part);
    }
    top += InspectPopover::CHIP_HEIGHT + InspectPopover::GAP;

    if let Some(font) = &popover.font {
        scene.push(text(
            &font.family,
            Point::new(left, top),
            InspectPopover::LINE,
            255,
        ));
        let at = left + InspectPopover::text_width(&font.family) + InspectPopover::GAP;
        scene.push(text(
            &font.detail,
            Point::new(at, top),
            InspectPopover::LINE,
            FAINT,
        ));
        top += InspectPopover::LINE + InspectPopover::GAP;
    }

    let mut x = left;
    for swatch in &popover.swatches {
        if let Some(color) = css_color(&swatch.value) {
            let square = Rect::new(
                x,
                top + (InspectPopover::LINE - InspectPopover::SWATCH) / 2.0,
                InspectPopover::SWATCH,
                InspectPopover::SWATCH,
            );
            scene.push(Item::screen(
                RectDraw::filled(square, color)
                    .with_corner_radius(SWATCH_RADIUS)
                    .with_stroke(Stroke::new(SWATCH_EDGE, 1.0, StrokeAlign::Inside)),
            ));
        }
        let label = format!("{} {}", swatch.label, swatch.value);
        let at = x + InspectPopover::SWATCH + InspectPopover::SWATCH_GAP;
        scene.push(text(
            &label,
            Point::new(at, top),
            InspectPopover::LINE,
            LABEL,
        ));
        x += InspectPopover::swatch_width(swatch) + InspectPopover::SWATCH_SPACING;
    }
}

/// A computed CSS colour, which the engine always gives as `rgb(r, g, b)` or
/// `rgba(r, g, b, a)`. `None` for anything else.
fn css_color(value: &str) -> Option<Color> {
    let value = value.trim();
    let inner = value
        .strip_prefix("rgba(")
        .or_else(|| value.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let parts: Vec<&str> = inner
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|part| !part.is_empty())
        .collect();
    let [r, g, b, rest @ ..] = parts.as_slice() else {
        return None;
    };
    let channel = |part: &str| {
        part.parse::<f32>()
            .ok()
            .map(|v| v.clamp(0.0, 255.0).round() as u8)
    };
    let alpha = match rest {
        [] => 255,
        [a] => (a.parse::<f32>().ok()?.clamp(0.0, 1.0) * 255.0).round() as u8,
        _ => return None,
    };
    Some(Color::rgba(channel(r)?, channel(g)?, channel(b)?, alpha))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computed_colours_parse_and_other_text_does_not() {
        let rows = [
            ("rgb(17, 24, 39)", Some(Color::rgb(17, 24, 39))),
            (
                "rgba(59, 130, 246, 0.5)",
                Some(Color::rgba(59, 130, 246, 128)),
            ),
            ("rgb(0 0 0 / 1)", Some(Color::rgb(0, 0, 0))),
            ("red", None),
            ("rgb(1, 2)", None),
        ];
        for (text, want) in rows {
            assert_eq!(css_color(text), want, "{text}");
        }
    }
}
