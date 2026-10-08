//! Groups: a tinted background, a border and a title above the top-left
//! corner. The border keeps its pixel size at any zoom, and the title does
//! down to half zoom.

use specular_doc::{Document, Entity, Group, Kind};

use super::editing;
use super::frame::Frame;
use super::page::title_above;
use super::palette::{self, Palette, Role};
use crate::{Color, Colors, Item, RectDraw, Scene, Stroke, StrokeAlign};

const CORNER_RADIUS: f32 = 2.0;
const BORDER_WIDTH: f32 = 1.5;

/// How a group looks: its tint, border and title colours.
struct Look {
    fill: Color,
    border: Color,
    title: Color,
}

fn look(colors: &Colors, group: &Group) -> Look {
    match &group.color {
        None => Look {
            fill: colors.group_fill,
            border: colors.group_border,
            title: colors.group_title,
        },
        Some(color) => {
            let ink = palette::resolve(color, Palette::Vivid, Role::Fill, colors);
            let (toward, kept) = colors.group_border_mix;
            Look {
                fill: palette::with_alpha(ink, colors.group_tint),
                border: mix(ink, toward, kept),
                title: colors.group_tinted_title,
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
    let fill = look(frame.colors, group).fill;
    let rect =
        RectDraw::filled(frame.screen_rect(entity.rect), fill).with_corner_radius(CORNER_RADIUS);
    scene.push(Item::screen(rect));
}

/// A group's border and title, in front of its members.
pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, group: &Group, scene: &mut Scene) {
    let on_screen = frame.screen_rect(entity.rect);
    let Look { border, title, .. } = look(frame.colors, group);
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
