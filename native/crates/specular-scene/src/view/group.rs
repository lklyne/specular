//! Groups: a tinted background, a border and a title above the top-left
//! corner. The border keeps its pixel size at any zoom, and the title does
//! down to half zoom.

use specular_doc::{Document, Entity, Group, Kind};

use super::editing;
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

/// How a group looks: its tint, border and title colours.
struct Look {
    fill: Color,
    border: Color,
    title: Color,
}

fn look(group: &Group) -> Look {
    match &group.color {
        None => Look {
            fill: PLAIN_FILL,
            border: PLAIN_BORDER,
            title: PLAIN_TITLE,
        },
        Some(color) => {
            let ink = palette::resolve(color, Palette::Vivid, Role::Fill);
            Look {
                fill: palette::with_alpha(ink, TINT_ALPHA),
                border: mix(ink, BORDER_MIX, BORDER_INK),
                title: TINTED_TITLE,
            }
        }
    }
}

/// The groups to tint, outermost first and in stack order within a depth.
/// Tints are drawn before every entity, as Electron's group fill sits below
/// everything it holds and everything that overlaps it, so an item being
/// dragged onto a group is never washed out by it.
pub(crate) fn backgrounds(document: &Document) -> Vec<&Entity> {
    let mut groups: Vec<&Entity> = document
        .entities()
        .filter(|entity| matches!(entity.kind, Kind::Group(_)))
        .collect();
    // Stable, and `entities` runs in stack order.
    groups.sort_by_key(|group| document.ancestors(&group.id).count());
    groups
}

/// The tint behind a group.
pub(crate) fn draw_background(frame: &Frame<'_>, entity: &Entity, scene: &mut Scene) {
    let Kind::Group(group) = &entity.kind else {
        return;
    };
    let fill = look(group).fill;
    let rect =
        RectDraw::filled(frame.screen_rect(entity.rect), fill).with_corner_radius(CORNER_RADIUS);
    scene.push(Item::screen(rect));
}

/// A group's border and title, in front of its members.
pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, group: &Group, scene: &mut Scene) {
    let on_screen = frame.screen_rect(entity.rect);
    let Look { border, title, .. } = look(group);
    let stroke = Stroke::new(border, BORDER_WIDTH, StrokeAlign::Inside);
    scene.push(Item::screen(
        RectDraw::outlined(on_screen, stroke).with_corner_radius(CORNER_RADIUS),
    ));
    let id = &entity.id;
    if let Some(shown) = frame.app.editing_text(id) {
        editing::selection(frame, id, None, scene);
        scene.extend(editing::edited_line(frame, shown, title));
        editing::caret(frame, id, None, title, scene);
        return;
    }
    match entity.label.as_deref() {
        Some(label) if !label.is_empty() => scene.push(title_above(frame, on_screen, label, title)),
        Some(_) | None => {}
    }
}

/// `amount` of `a` with the rest `b`, channel by channel.
fn mix(a: Color, b: Color, amount: f32) -> Color {
    let channel =
        |a: u8, b: u8| (f32::from(a) * amount + f32::from(b) * (1.0 - amount)).round() as u8;
    Color::rgb(channel(a.r, b.r), channel(a.g, b.g), channel(a.b, b.b))
}
