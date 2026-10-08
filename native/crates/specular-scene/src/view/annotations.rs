//! Comments on the canvas: the dashed frame of a region comment and the
//! count pill of everything else, from `App::comment_marks`, then the marker
//! of the draft being written. All of it is screen-sized chrome.

use specular_interact::{
    CommentMark, FOCUS_RING_OUTSET, FOCUS_RING_STROKE, MarkShape, PILL_HEIGHT, ScreenRect,
};

use super::comment_draft;
use super::frame::Frame;
use super::palette;
use super::shape_path::Silhouette;
use crate::{
    Color, Dash, Item, PageBand, PathDraw, PathStroke, Point, Rect, RectDraw, Scene, Stroke,
    StrokeAlign, TextAlign, TextRun, VerticalAlign,
};

const REGION_COLOR: Color = Color::rgb(0xff, 0x63, 0x7e);
const REGION_RADIUS: f32 = 4.0;
const REGION_STROKE: f32 = 2.0;
const REGION_DASH: Dash = Dash { on: 6.0, off: 4.0 };
const REGION_STROKE_ALPHA: f32 = 0.7;
const REGION_FILL_ALPHA: f32 = 0.05;
/// A resting region is faint, so it does not fight the page under it.
const REGION_OPACITY: f32 = 0.5;
/// A focused region is solid and its fill doubles, as Electron's hovered
/// region is.
const FOCUSED_FILL_ALPHA: f32 = 0.1;

/// The blue of pills, their focus ring and the draft markers.
pub(super) const BLUE: Color = Color::rgb(0x2b, 0x7f, 0xff);
const PILL_BORDER: Color = Color::rgba(0x8e, 0xc5, 0xff, 230);
const PILL_TEXT: f32 = 10.0;

pub(crate) fn draw(frame: &Frame<'_>, scene: &mut Scene) {
    let marks = frame.app.comment_marks();
    // Frames go under pills so a pill is never covered by a frame.
    for mark in &marks {
        if let MarkShape::Region(on_screen) = mark.shape {
            region(frame, mark, on_screen, scene);
        }
    }
    for mark in &marks {
        if let MarkShape::Badge(on_screen) = mark.shape {
            pill(frame, mark, on_screen, scene);
        }
    }
    comment_draft::marker(frame, scene);
}

/// A scene rect from a screen rect of the interaction layer.
pub(super) fn scene_rect(on_screen: ScreenRect) -> Rect {
    Rect::new(
        on_screen.min.x,
        on_screen.min.y,
        on_screen.size.x,
        on_screen.size.y,
    )
}

fn region(frame: &Frame<'_>, mark: &CommentMark, on_screen: ScreenRect, scene: &mut Scene) {
    let on_screen = scene_rect(on_screen);
    if !frame.sees_screen(on_screen) {
        return;
    }
    let fill = if mark.focused {
        FOCUSED_FILL_ALPHA
    } else {
        REGION_FILL_ALPHA
    };
    let opacity = if mark.focused { 1.0 } else { REGION_OPACITY };
    let items = region_items(on_screen, REGION_COLOR, REGION_STROKE_ALPHA, fill)
        .map(|item| item.with_opacity(opacity));
    // On a page, a region shows only through the page it scrolls with.
    match mark.clip.map(scene_rect) {
        Some(page) => {
            let band = PageBand {
                page,
                reach: specular_interact::PAGE_FADE,
            };
            for item in items {
                band.through(item, on_screen, &mut scene.items);
            }
        }
        None => scene.extend(items),
    }
}

/// A dashed, tinted, rounded region over `on_screen`: a fill and its border.
pub(crate) fn region_items(
    on_screen: Rect,
    color: Color,
    stroke_alpha: f32,
    fill_alpha: f32,
) -> impl Iterator<Item = Item> {
    let fill = RectDraw::filled(on_screen, palette::with_alpha(color, fill_alpha))
        .with_corner_radius(REGION_RADIUS);
    // The border is inside the rect, as a CSS border is.
    let inner = on_screen.outset(-REGION_STROKE / 2.0);
    let border = PathDraw {
        commands: Silhouette::Rect(REGION_RADIUS).into_path(inner),
        fill: None,
        stroke: Some(
            PathStroke::new(palette::with_alpha(color, stroke_alpha), REGION_STROKE)
                .dashed(REGION_DASH),
        ),
    };
    [Item::screen(fill), Item::screen(border)].into_iter()
}

/// A count pill, ringed when its comment has the focus.
fn pill(frame: &Frame<'_>, mark: &CommentMark, on_screen: ScreenRect, scene: &mut Scene) {
    let pill = scene_rect(on_screen);
    let ring = pill.outset(FOCUS_RING_OUTSET);
    if !frame.sees_screen(if mark.focused { ring } else { pill }) {
        return;
    }
    if mark.focused {
        let stroke = Stroke::new(BLUE, FOCUS_RING_STROKE, StrokeAlign::Inside);
        scene.push(Item::screen(
            RectDraw::outlined(ring, stroke).with_corner_radius(ring.height / 2.0),
        ));
    }
    scene.push(Item::screen(
        RectDraw::filled(pill, BLUE)
            .with_corner_radius(PILL_HEIGHT / 2.0)
            .with_stroke(Stroke::new(PILL_BORDER, 1.0, StrokeAlign::Inside)),
    ));
    let middle = Point::new(pill.x + pill.width / 2.0, pill.y + pill.height / 2.0);
    scene.push(Item::screen(TextRun {
        weight: 600,
        line_height: PILL_TEXT,
        align: TextAlign::Centre,
        vertical_align: VerticalAlign::Middle,
        ..TextRun::new(mark.count.to_string(), middle, PILL_TEXT, Color::WHITE)
    }));
}
