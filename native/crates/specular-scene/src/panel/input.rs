//! A text field: its box, and the line of text in it with the selection,
//! the input method's underline and the caret over what is being edited.

use specular_doc::TextFont;
use specular_interact::panel::builtin::{FIELD_LINE, Input, Node, PanelRect};

use super::colors::{FIELD_BORDER, INPUT, INPUT_RING, TEXT, TEXT_MUTED};
use super::node::line;
use super::rect;
use crate::view::palette;
use crate::{Item, Rect, RectDraw, Stroke, StrokeAlign};

/// How far below a line's top its underline sits, as a fraction of the line.
const UNDERLINE_DROP: f32 = 0.9;

/// The white box with a zinc outline, and when `ring` the ring around it
/// while it has the keys (`focus:ring-1 focus:ring-blue-500/40`). A name
/// edited in place keeps only its outline.
pub(super) fn chrome(node: &Node, ring: bool, out: &mut Vec<Item>) {
    let area = rect(node.rect);
    let edge = Stroke::new(FIELD_BORDER, 1.0, StrokeAlign::Inside);
    out.push(Item::screen(
        RectDraw::filled(area, INPUT)
            .with_corner_radius(node.radius)
            .with_stroke(edge),
    ));
    if ring && node.state.on {
        let ring = Stroke::new(INPUT_RING, 1.0, StrokeAlign::Outside);
        out.push(Item::screen(
            RectDraw::outlined(area, ring).with_corner_radius(node.radius),
        ));
    }
}

/// The line of `input`, cut off at the box it sits in.
pub(super) fn draw(input: &Input, out: &mut Vec<Item>) {
    let area = rect(input.area);
    let clipped = |item: Item| item.clipped(area);
    if let Some(focus) = &input.focus {
        for selected in &focus.selection {
            let fill = RectDraw::filled(rect(*selected), palette::TEXT_SELECTION);
            out.push(clipped(Item::screen(fill)));
        }
    }
    if input.text.is_empty() {
        if let Some(hint) = &input.hint {
            out.push(clipped(shown(hint, input.area, 0.0, TEXT_MUTED)));
        }
    } else {
        out.push(clipped(shown(&input.text, input.area, input.scroll, TEXT)));
    }
    let Some(focus) = &input.focus else {
        return;
    };
    for composing in &focus.composition {
        let top = composing.y + (composing.height.min(FIELD_LINE)) * UNDERLINE_DROP;
        let under = Rect::new(composing.x, top.round(), composing.width, 1.0);
        out.push(clipped(Item::screen(RectDraw::filled(under, TEXT))));
    }
    if let Some(caret) = focus.caret {
        out.push(clipped(Item::screen(RectDraw::filled(rect(caret), TEXT))));
    }
}

/// `text` from the left of `area`, scrolled left by `scroll`.
fn shown(text: &str, area: PanelRect, scroll: f32, color: crate::Color) -> Item {
    let moved = PanelRect::new(area.x - scroll, area.y, area.width + scroll, area.height);
    line(
        text,
        moved,
        specular_doc::TextAlign::Left,
        TextFont::Sans,
        400,
        color,
    )
}
