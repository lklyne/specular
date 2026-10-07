//! Comments on the canvas: a dashed region for a region comment, a count
//! badge for a comment on a page or one of its elements, and a dot for a
//! comment on a canvas point. All of it is screen-sized chrome.

use glam::DVec2;
use specular_doc::{Annotation, AnnotationAnchor, AnnotationStatus, EntityId};
use specular_interact::region_on_canvas;

use super::frame::Frame;
use super::palette;
use super::shape_path::Silhouette;
use crate::{
    Color, Dash, EllipseDraw, Item, PathDraw, PathStroke, Point, Rect, RectDraw, Scene, Stroke,
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

const BADGE_FILL: Color = Color::rgb(0x2b, 0x7f, 0xff);
const BADGE_BORDER: Color = Color::rgba(0x8e, 0xc5, 0xff, 230);
const BADGE_HEIGHT: f32 = 26.0;
/// Badge width with a one-digit count, and what each further digit adds.
const BADGE_WIDTH: f32 = 26.0;
const BADGE_DIGIT: f32 = 6.0;
const BADGE_TEXT: f32 = 10.0;
/// How far inside the page's right and top edges a badge sits.
const BADGE_INSET: f32 = 8.0;
/// A badge's centre stays this far from the page's top and bottom.
const BADGE_EDGE: f32 = 10.0;

const POINT_SIZE: f32 = 12.0;
const POINT_STROKE: f32 = 2.0;

pub(crate) fn draw(frame: &Frame<'_>, scene: &mut Scene) {
    for annotation in frame.app.document().annotations() {
        match annotation.status {
            AnnotationStatus::Pending | AnnotationStatus::Acknowledged => {}
            AnnotationStatus::Resolved | AnnotationStatus::Dismissed => continue,
        }
        match &annotation.anchor {
            AnnotationAnchor::Region(_) => {
                if let Some(region) = region_on_canvas(frame.app, annotation) {
                    let on_screen = frame.screen_rect(region);
                    if frame.sees_screen(on_screen) {
                        scene.extend(
                            region_items(
                                on_screen,
                                REGION_COLOR,
                                REGION_STROKE_ALPHA,
                                REGION_FILL_ALPHA,
                            )
                            .map(|item| item.with_opacity(REGION_OPACITY)),
                        );
                    }
                }
            }
            AnnotationAnchor::Page {
                page_id: page,
                offset_y,
                ..
            } => badge(frame, page, Some(*offset_y), annotation, scene),
            AnnotationAnchor::Element { page_id: page, .. } => {
                badge(frame, page, None, annotation, scene);
            }
            AnnotationAnchor::Canvas { canvas_x, canvas_y } => {
                let centre = frame.screen_point(DVec2::new(*canvas_x, *canvas_y));
                let dot = Rect::new(
                    centre.x - POINT_SIZE / 2.0,
                    centre.y - POINT_SIZE / 2.0,
                    POINT_SIZE,
                    POINT_SIZE,
                );
                if frame.sees_screen(dot) {
                    scene.push(Item::screen(
                        EllipseDraw::filled(dot, palette::with_alpha(BADGE_FILL, 0.2)).with_stroke(
                            Stroke::new(
                                palette::with_alpha(BADGE_FILL, 0.95),
                                POINT_STROKE,
                                StrokeAlign::Inside,
                            ),
                        ),
                    ));
                }
            }
        }
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

/// The count badge inside the right edge of `page`: at `offset_y` of the way
/// down it, or in the top corner for a comment on an element, whose live
/// position the page has not reported.
fn badge(
    frame: &Frame<'_>,
    page: &EntityId,
    offset_y: Option<f64>,
    annotation: &Annotation,
    scene: &mut Scene,
) {
    let Some(placement) = frame.app.page_placement(page) else {
        return;
    };
    let on_screen = frame.screen_rect(placement.rect);
    let count = (1 + annotation.replies.len()).to_string();
    let width = BADGE_WIDTH + BADGE_DIGIT * count.len().saturating_sub(1) as f32;
    let centre_y = match offset_y {
        Some(offset) => {
            let wanted = on_screen.y + offset as f32 * on_screen.height;
            let (low, high) = (on_screen.y + BADGE_EDGE, on_screen.bottom() - BADGE_EDGE);
            wanted.min(high).max(low)
        }
        None => on_screen.y + BADGE_INSET + BADGE_HEIGHT / 2.0,
    };
    let pill = Rect::new(
        on_screen.right() - BADGE_INSET - width,
        centre_y - BADGE_HEIGHT / 2.0,
        width,
        BADGE_HEIGHT,
    );
    if !frame.sees_screen(pill) {
        return;
    }
    scene.push(Item::screen(
        RectDraw::filled(pill, BADGE_FILL)
            .with_corner_radius(BADGE_HEIGHT / 2.0)
            .with_stroke(Stroke::new(BADGE_BORDER, 1.0, StrokeAlign::Inside)),
    ));
    scene.push(Item::screen(TextRun {
        weight: 600,
        line_height: BADGE_TEXT,
        align: TextAlign::Centre,
        vertical_align: VerticalAlign::Middle,
        ..TextRun::new(
            count,
            Point::new(pill.x + width / 2.0, centre_y),
            BADGE_TEXT,
            Color::WHITE,
        )
    }));
}
