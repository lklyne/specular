//! Pages: the live frame, and around it a border and a title.

use specular_doc::{DeviceKind, DeviceShell, Entity, Page};
use specular_interact::{PageState, title_scale};

use super::frame::{Frame, canvas_rect};
use crate::{
    Item, PageDraw, Rect, RectDraw, Scene, ShadowDraw, Stroke, StrokeAlign, TextOverflow, TextRun,
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

/// The drop and blur of a device frame's shadow, in canvas units.
const SHELL_SHADOW: f32 = 16.0;
/// The ring in the bezel around the screen, in logical pixels.
const SCREEN_RING_WIDTH: f32 = 1.5;
/// A phone's notch: its size, its corner radius and its gap from the top
/// of the screen, in canvas units.
const NOTCH: (f32, f32, f32, f32) = (126.0, 37.0, 18.5, 8.0);
/// The home indicator's height, in canvas units.
const INDICATOR_HEIGHT: f32 = 4.0;

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, page: &Page, scene: &mut Scene) {
    let screen = canvas_rect(entity.rect);
    // A screenshot of the page is the page alone, so the frame is chrome.
    let shell = page.shell().filter(|_| frame.chrome);
    let outer = shell.map_or(screen, |shell| shell_rect(screen, &shell));
    if let Some(shell) = &shell {
        let radius = shell.corner_radius as f32;
        scene.push(Item::canvas(ShadowDraw {
            rect: Rect::new(outer.x, outer.y + SHELL_SHADOW, outer.width, outer.height),
            corner_radius: radius,
            blur: SHELL_SHADOW,
            color: frame.colors.device_shadow,
        }));
        scene.push(Item::canvas(
            RectDraw::filled(outer, frame.colors.device_bezel).with_corner_radius(radius),
        ));
    }
    let screen_radius = shell.map_or(CORNER_RADIUS, |shell| shell.screen_corner_radius as f32);
    scene.push(Item::canvas(PageDraw {
        page: entity.id.clone(),
        rect: screen,
        corner_radius: screen_radius,
    }));
    if !frame.chrome {
        return;
    }
    let zoom = frame.zoom();
    let on_screen = frame.project(outer);
    if let Some(shell) = &shell {
        decorate(frame, screen, outer, shell, scene);
        let ring = Stroke::new(
            frame.colors.device_screen_ring,
            SCREEN_RING_WIDTH,
            StrokeAlign::Outside,
        );
        scene.push(Item::screen(
            RectDraw::outlined(frame.project(screen), ring)
                .with_corner_radius(screen_radius * zoom),
        ));
    }
    // A ring just outside the frame, one pixel wide at any zoom.
    let border = Stroke::new(frame.colors.page_border, BORDER_WIDTH, StrokeAlign::Outside);
    let radius = shell.map_or(CORNER_RADIUS, |shell| shell.corner_radius as f32);
    scene.push(Item::screen(
        RectDraw::outlined(on_screen, border).with_corner_radius(radius * zoom),
    ));
    let title = title(entity, page, frame.app.page_state(&entity.id));
    if !title.is_empty() {
        scene.push(title_above(
            frame,
            on_screen,
            &title,
            frame.colors.muted_text,
        ));
    }
}

/// The shell around `screen`, in canvas units.
fn shell_rect(screen: Rect, shell: &DeviceShell) -> Rect {
    let insets = shell.insets;
    let (left, top) = (insets.left as f32, insets.top as f32);
    Rect::new(
        screen.x - left,
        screen.y - top,
        screen.width + left + insets.right as f32,
        screen.height + top + insets.bottom as f32,
    )
}

/// A phone's notch, and the home indicator in a phone's or a tablet's
/// bottom bezel.
fn decorate(frame: &Frame<'_>, screen: Rect, outer: Rect, shell: &DeviceShell, scene: &mut Scene) {
    let (phone, tablet) = match shell.kind {
        DeviceKind::Phone => (true, false),
        DeviceKind::Tablet => (false, true),
        DeviceKind::Plain => return,
    };
    let centre = outer.x + outer.width / 2.0;
    if phone && shell.screen_corner_radius > 0.0 && !shell.landscape {
        let (width, height, radius, gap) = NOTCH;
        let notch = Rect::new(centre - width / 2.0, screen.y + gap, width, height);
        scene.push(Item::canvas(
            RectDraw::filled(notch, frame.colors.device_notch).with_corner_radius(radius),
        ));
    }
    let width = if phone && !shell.landscape {
        120.0
    } else {
        100.0
    };
    let bezel = shell.insets.bottom as f32;
    let lift = (bezel / 2.0 - if tablet { 3.0 } else { 4.0 }).max(4.0);
    let indicator = Rect::new(
        centre - width / 2.0,
        outer.y + outer.height - lift - INDICATOR_HEIGHT,
        width,
        INDICATOR_HEIGHT,
    );
    scene.push(Item::canvas(
        RectDraw::filled(indicator, frame.colors.device_indicator)
            .with_corner_radius(INDICATOR_HEIGHT / 2.0),
    ));
}

/// What the title line of a page says. A label the user gave wins. Without
/// one it reads `Title — address` once the page has a title, and the address
/// alone before that; while a load is in flight it starts `Loading… `.
fn title(entity: &Entity, page: &Page, state: Option<&PageState>) -> String {
    let label = entity.label.as_deref().filter(|label| !label.is_empty());
    let loading = state.is_some_and(|state| state.loading);
    let line = if let Some(label) = label {
        label.to_owned()
    } else {
        let url = state.and_then(|state| state.url.as_deref());
        let address = address(url.unwrap_or(&page.url));
        match state
            .map(|state| state.title.trim())
            .filter(|title| !title.is_empty())
        {
            Some(title) => format!("{title} \u{2014} {address}"),
            None => address.to_owned(),
        }
    };
    if loading && !line.is_empty() {
        format!("Loading\u{2026} {line}")
    } else {
        line
    }
}

/// An address without its scheme or trailing slash.
fn address(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.trim_end_matches('/')
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
