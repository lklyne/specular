//! Groups: a tinted background, a border and a title above the top-left
//! corner. The border and title keep their pixel size at any zoom.

use specular_doc::{Entity, Group};

use super::frame::Frame;
use super::page::title_above;
use super::palette::{self, Palette, Role};
use crate::{Color, Item, RectDraw, Scene, Stroke, StrokeAlign};

const CORNER_RADIUS: f32 = 2.0;
const BORDER_WIDTH: f32 = 1.5;
const PLAIN_FILL: Color = Color::rgba(244, 244, 245, 115);
const PLAIN_BORDER: Color = Color::rgba(113, 113, 122, 64);
const PLAIN_TITLE: Color = Color::rgb(0x3f, 0x3f, 0x46);
const TINTED_TITLE: Color = Color::rgb(0x18, 0x18, 0x1b);
/// A coloured group's background is its ink at this alpha.
const TINT_ALPHA: f32 = 0.3;
/// A coloured group's border is its ink mixed this far with [`BORDER_MIX`].
const BORDER_INK: f32 = 0.78;
const BORDER_MIX: Color = Color::rgb(0xa1, 0x62, 0x07);

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, group: &Group, scene: &mut Scene) {
    let on_screen = frame.screen_rect(entity.rect);
    let (fill, border, title) = match &group.color {
        None => (PLAIN_FILL, PLAIN_BORDER, PLAIN_TITLE),
        Some(color) => {
            let ink = palette::resolve(color, Palette::Vivid, Role::Fill);
            (
                palette::with_alpha(ink, TINT_ALPHA),
                mix(ink, BORDER_MIX, BORDER_INK),
                TINTED_TITLE,
            )
        }
    };
    scene.push(Item::screen(
        RectDraw::filled(on_screen, fill)
            .with_corner_radius(CORNER_RADIUS)
            .with_stroke(Stroke::new(border, BORDER_WIDTH, StrokeAlign::Inside)),
    ));
    match entity.label.as_deref() {
        Some(label) if !label.is_empty() => scene.push(title_above(on_screen, label, title)),
        Some(_) | None => {}
    }
}

/// `amount` of `a` with the rest `b`, channel by channel.
fn mix(a: Color, b: Color, amount: f32) -> Color {
    let channel =
        |a: u8, b: u8| (f32::from(a) * amount + f32::from(b) * (1.0 - amount)).round() as u8;
    Color::rgb(channel(a.r, b.r), channel(a.g, b.g), channel(a.b, b.b))
}
