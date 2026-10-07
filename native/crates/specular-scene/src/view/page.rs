//! Pages: the live frame, and around it a border and a title.

use specular_doc::{Entity, Page};

use super::frame::{Frame, canvas_rect};
use super::palette;
use crate::{Item, PageDraw, Rect, RectDraw, Scene, Stroke, StrokeAlign, TextRun, VerticalAlign};

/// Corner radius of a page's frame, in canvas units.
pub(crate) const CORNER_RADIUS: f32 = 8.0;
const BORDER_WIDTH: f32 = 1.0;
/// The title's line box and the gap under it, in logical pixels. A group's
/// title sits the same way above its rect.
pub(crate) const TITLE_LINE: f32 = 16.5;
pub(crate) const TITLE_GAP: f32 = 4.0;
pub(crate) const TITLE_SIZE: f32 = 11.0;
pub(crate) const TITLE_WEIGHT: u16 = 500;

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, page: &Page, scene: &mut Scene) {
    scene.push(Item::canvas(PageDraw {
        page: entity.id.clone(),
        rect: canvas_rect(entity.rect),
        corner_radius: CORNER_RADIUS,
    }));
    if !frame.chrome {
        return;
    }
    let on_screen = frame.screen_rect(entity.rect);
    // A ring just outside the frame, one pixel wide at any zoom.
    let border = Stroke::new(palette::PAGE_BORDER, BORDER_WIDTH, StrokeAlign::Outside);
    scene.push(Item::screen(
        RectDraw::outlined(on_screen, border).with_corner_radius(CORNER_RADIUS * frame.zoom()),
    ));
    let title = title(entity, page);
    if !title.is_empty() {
        scene.push(title_above(on_screen, title, palette::MUTED_TEXT));
    }
}

/// What a page is called on the canvas: its label, or its address without
/// the scheme.
fn title<'a>(entity: &'a Entity, page: &'a Page) -> &'a str {
    match entity.label.as_deref() {
        Some(label) if !label.is_empty() => label,
        Some(_) | None => {
            let address = page
                .url
                .split_once("://")
                .map_or(&*page.url, |(_, rest)| rest);
            address.trim_end_matches('/')
        }
    }
}

/// One line of chrome text above the top-left corner of `on_screen`, cut off
/// at the rect's width.
pub(crate) fn title_above(on_screen: Rect, text: &str, color: crate::Color) -> Item {
    let top = on_screen.y - TITLE_GAP - TITLE_LINE;
    let run = TextRun {
        box_height: Some(TITLE_LINE),
        weight: TITLE_WEIGHT,
        vertical_align: VerticalAlign::Middle,
        ..TextRun::new(text, crate::Point::new(on_screen.x, top), TITLE_SIZE, color)
    };
    Item::screen(run).clipped(Rect::new(
        on_screen.x,
        top,
        on_screen.width,
        TITLE_LINE + TITLE_GAP,
    ))
}
