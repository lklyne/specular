//! Pages: the live frame, and around it a border and a title.

use specular_doc::{Entity, Page};
use specular_interact::title_scale;

use super::frame::{Frame, canvas_rect};
use super::palette;
use crate::{
    Item, PageDraw, Rect, RectDraw, Scene, Stroke, StrokeAlign, TextOverflow, TextRun,
    VerticalAlign,
};

/// Corner radius of a page's frame, in canvas units.
pub(crate) const CORNER_RADIUS: f32 = 8.0;
const BORDER_WIDTH: f32 = 1.0;
/// The title's line box and the gap under it, in logical pixels. A group's
/// title sits the same way above its rect.
pub(crate) const TITLE_LINE: f32 = specular_interact::TITLE_LINE;
pub(crate) const TITLE_GAP: f32 = specular_interact::TITLE_GAP;
pub(crate) const TITLE_SIZE: f32 = specular_interact::TITLE_SIZE;
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
        scene.push(title_above(frame, on_screen, title, palette::MUTED_TEXT));
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

/// One line of chrome text above the top-left corner of `on_screen`. It is
/// never wider than the rect: a longer title ends in an ellipsis. It keeps
/// its pixel size down to half zoom and shrinks with the canvas below that,
/// so zoomed out it does not outgrow what it names or run into its
/// neighbours. Small enough, the renderer stops drawing it.
pub(crate) fn title_above(
    frame: &Frame<'_>,
    on_screen: Rect,
    text: &str,
    color: crate::Color,
) -> Item {
    let scale = title_scale(frame.zoom());
    let (line, gap) = (TITLE_LINE * scale, TITLE_GAP * scale);
    let top = on_screen.y - gap - line;
    let run = TextRun {
        wrap_width: Some(on_screen.width),
        overflow: TextOverflow::Ellipsis,
        box_height: Some(line),
        weight: TITLE_WEIGHT,
        vertical_align: VerticalAlign::Middle,
        ..TextRun::new(
            text,
            crate::Point::new(on_screen.x, top),
            TITLE_SIZE * scale,
            color,
        )
    };
    Item::screen(run).clipped(Rect::new(on_screen.x, top, on_screen.width, line + gap))
}
