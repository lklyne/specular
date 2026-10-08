//! The popover's text, size and place (`InspectPopoverLayer.tsx`,
//! `inspect-popover-position.ts`).
//!
//! The Electron popover is measured in the DOM. Here its size is computed
//! from the rows it holds and fixed metrics, so the same model gives the
//! same box in a test, in a snapshot and on screen.

use glam::Vec2;
use specular_core::InspectedNode;

use super::{InspectFont, InspectPopover, InspectSwatch};
use crate::ScreenRect;

/// Space between the window's edge and the popover.
const VIEWPORT_PADDING: f32 = 8.0;
/// Space between the target and the popover.
const TARGET_GAP: f32 = 6.0;
/// The most classes the remainder names.
const MAX_CLASSES: usize = 3;
/// A computed background that is no background.
const TRANSPARENT: &str = "rgba(0, 0, 0, 0)";

impl InspectPopover {
    /// The text size, in logical pixels.
    pub const FONT_SIZE: f32 = 11.0;
    /// The width of one character of the 11 px monospace face, as the renderer sets it.
    pub const ADVANCE: f32 = 6.7;
    /// The height of a row of plain text.
    pub const LINE: f32 = 14.0;
    /// The height of the row holding the tag chip.
    pub const CHIP_HEIGHT: f32 = 18.0;
    /// The space left and right of the tag inside its chip.
    pub const CHIP_PADDING: f32 = 6.0;
    /// The space between the card's edge and its rows.
    pub const PADDING: f32 = 6.0;
    /// The space between rows, and between the parts of a row.
    pub const GAP: f32 = 6.0;
    /// The side of a colour square.
    pub const SWATCH: f32 = 10.0;
    /// The space between a colour square and its label, and between the
    /// colours.
    pub const SWATCH_GAP: f32 = 4.0;
    /// The space between two colours.
    pub const SWATCH_SPACING: f32 = 10.0;
    /// The widest the card grows.
    pub const MAX_WIDTH: f32 = 360.0;

    /// The width of `text` set in the card's face.
    pub fn text_width(text: &str) -> f32 {
        text.chars().count() as f32 * Self::ADVANCE
    }

    /// The width of one colour: its square, then `label value`.
    pub fn swatch_width(swatch: &InspectSwatch) -> f32 {
        let text = format!("{} {}", swatch.label, swatch.value);
        Self::SWATCH + Self::SWATCH_GAP + Self::text_width(&text)
    }

    fn rows(&self) -> [Option<f32>; 3] {
        let mut chip = Self::text_width(&self.tag) + 2.0 * Self::CHIP_PADDING;
        for part in [&self.remainder, &self.size] {
            if !part.is_empty() {
                chip += Self::GAP + Self::text_width(part);
            }
        }
        let font = self.font.as_ref().map(|font| {
            Self::text_width(&font.family) + Self::GAP + Self::text_width(&font.detail)
        });
        let swatches = (!self.swatches.is_empty()).then(|| {
            let widths: f32 = self.swatches.iter().map(Self::swatch_width).sum();
            widths + Self::SWATCH_SPACING * (self.swatches.len() - 1) as f32
        });
        [Some(chip), font, swatches]
    }

    /// The card's size: its widest row and the rows stacked, with padding.
    pub fn size_of(&self) -> Vec2 {
        let rows = self.rows();
        let widest = rows.iter().flatten().fold(0.0_f32, |most, w| most.max(*w));
        let mut height = Self::CHIP_HEIGHT;
        for row in &rows[1..] {
            if row.is_some() {
                height += Self::GAP + Self::LINE;
            }
        }
        Vec2::new(
            (widest + 2.0 * Self::PADDING).min(Self::MAX_WIDTH),
            height + 2.0 * Self::PADDING,
        )
    }

    /// The popover for `node`, put above `target` when it fits, else below,
    /// inside `viewport`.
    pub(super) fn of(node: &InspectedNode, target: ScreenRect, viewport: Vec2) -> Self {
        let mut popover = Self {
            rect: ScreenRect::new(0.0, 0.0, 0.0, 0.0),
            tag: if node.tag_name.is_empty() {
                "element".to_owned()
            } else {
                node.tag_name.clone()
            },
            remainder: remainder(node),
            size: format!(
                "{} \u{d7} {}",
                node.bounding_box.width, node.bounding_box.height
            ),
            font: font(node),
            swatches: swatches(node),
        };
        let size = popover.size_of();
        let at = place(target, size, viewport);
        popover.rect = ScreenRect { min: at, size };
        popover
    }
}

/// `#id.class.class`: the id and the first three classes.
fn remainder(node: &InspectedNode) -> String {
    let id = node
        .id_attribute
        .as_deref()
        .filter(|id| !id.is_empty())
        .map(|id| format!("#{id}"));
    let classes = node
        .classes
        .iter()
        .take(MAX_CLASSES)
        .map(|c| format!(".{c}"));
    id.into_iter().chain(classes).collect()
}

fn font(node: &InspectedNode) -> Option<InspectFont> {
    let families = node.style("font-family")?;
    let first = families.split(',').next().unwrap_or("").trim();
    let family = first.trim_matches(|c| c == '"' || c == '\'');
    if family.is_empty() {
        return None;
    }
    let detail = ["font-size", "font-weight"]
        .into_iter()
        .filter_map(|property| node.style(property).filter(|v| !v.is_empty()))
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    Some(InspectFont {
        family: family.to_owned(),
        detail,
    })
}

fn swatches(node: &InspectedNode) -> Vec<InspectSwatch> {
    let swatch = |label: &str, value: &str| InspectSwatch {
        label: label.to_owned(),
        value: value.to_owned(),
    };
    let color = node.style("color").filter(|v| !v.is_empty());
    let background = node
        .style("background")
        .filter(|v| !v.is_empty() && *v != TRANSPARENT);
    let mut out = Vec::new();
    out.extend(color.map(|value| swatch("text", value)));
    out.extend(background.map(|value| swatch("bg", value)));
    out
}

/// `placeInspectPopover`: the card's top-left. It lines up with the
/// target's left edge, clamped into the window, and sits above the target
/// when that leaves the window's padding, else below it.
pub(super) fn place(target: ScreenRect, popover: Vec2, viewport: Vec2) -> Vec2 {
    let max_left = (viewport.x - popover.x - VIEWPORT_PADDING).max(VIEWPORT_PADDING);
    let left = target.min.x.round().clamp(VIEWPORT_PADDING, max_left);
    let above = target.min.y.round() - popover.y - TARGET_GAP;
    let below = target.max().y.round() + TARGET_GAP;
    let max_top = (viewport.y - popover.y - VIEWPORT_PADDING).max(VIEWPORT_PADDING);
    let top = if above >= VIEWPORT_PADDING {
        above
    } else {
        below.clamp(VIEWPORT_PADDING, max_top)
    };
    Vec2::new(left, top)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_popover_sits_above_or_below_the_target_inside_the_window() {
        let window = Vec2::new(1200.0, 900.0);
        let card = Vec2::new(260.0, 84.0);
        let rows = [
            // Zoomed out: room above, so one gap over the target.
            (ScreenRect::new(325.0, 145.0, 12.5, 5.0), (325.0, 55.0)),
            // Zoomed in: the same.
            (ScreenRect::new(500.0, 320.0, 100.0, 40.0), (500.0, 230.0)),
            // No room above: below it, and pulled in from the right edge.
            (ScreenRect::new(1180.0, 20.0, 10.0, 10.0), (932.0, 36.0)),
            // No room above and none below: held off the bottom edge.
            (ScreenRect::new(4.0, 20.0, 10.0, 870.0), (8.0, 808.0)),
        ];
        for (target, want) in rows {
            assert_eq!(place(target, card, window), Vec2::from(want), "{target:?}");
        }
    }
}
