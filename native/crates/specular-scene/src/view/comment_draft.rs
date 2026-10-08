//! The comment being written: the marker on what it is about, and the
//! composer card its text is typed in. The composer goes over all other
//! chrome so nothing hides what is being typed.

use glam::DVec2;
use specular_doc::{AnnotationAnchor, EntityId};
use specular_interact::{element_on_canvas, left_its_page, page_clip, region_on_canvas};

use super::annotations::{BLUE, region_items};
use super::editing;
use super::frame::Frame;
use super::palette;
use crate::{
    Color, Dash, EllipseDraw, Item, PathDraw, PathStroke, Rect, RectDraw, Scene, Stroke,
    StrokeAlign, TextRun,
};

const POINT_SIZE: f32 = 12.0;
const POINT_STROKE: f32 = 2.0;
const POINT_STROKE_ALPHA: f32 = 0.95;
const POINT_FILL_ALPHA: f32 = 0.2;
const REGION_STROKE_ALPHA: f32 = 0.9;
const REGION_FILL_ALPHA: f32 = 0.1;
/// The outline of the element a draft is about.
const ELEMENT_COLOR: Color = Color::rgb(0x3b, 0x82, 0xf6);
const ELEMENT_STROKE_ALPHA: f32 = 0.95;
const ELEMENT_FILL_ALPHA: f32 = 0.14;
const ELEMENT_DASH: Dash = Dash { on: 3.0, off: 3.0 };

const CARD_RADIUS: f32 = 10.0;
const CARD_BORDER_ALPHA: f32 = 0.8;
const PLACEHOLDER: &str = "Add a comment";

/// What the draft is about, outlined the way Electron's draft marker is.
pub(crate) fn marker(frame: &Frame<'_>, scene: &mut Scene) {
    let app = frame.app;
    let Some(draft) = app.comment_draft() else {
        return;
    };
    // What the draft is on may have scrolled out of its page.
    if left_its_page(app, draft) {
        return;
    }
    let clip = page_clip(app, draft).map(|page| frame.screen_rect(page));
    let clipped = |item: Item| match clip {
        Some(clip) => item.clipped(clip),
        None => item,
    };
    match &draft.anchor {
        AnnotationAnchor::Canvas { canvas_x, canvas_y } => {
            let centre = frame.screen_point(DVec2::new(*canvas_x, *canvas_y));
            let dot = Rect::new(
                centre.x - POINT_SIZE / 2.0,
                centre.y - POINT_SIZE / 2.0,
                POINT_SIZE,
                POINT_SIZE,
            );
            if frame.sees_screen(dot) {
                let stroke = Stroke::new(
                    palette::with_alpha(BLUE, POINT_STROKE_ALPHA),
                    POINT_STROKE,
                    StrokeAlign::Inside,
                );
                scene.push(Item::screen(
                    EllipseDraw::filled(dot, palette::with_alpha(BLUE, POINT_FILL_ALPHA))
                        .with_stroke(stroke),
                ));
            }
        }
        AnnotationAnchor::Region(_) => {
            if let Some(region) = region_on_canvas(app, draft) {
                let on_screen = frame.screen_rect(region);
                if frame.sees_screen(on_screen) {
                    scene.extend(
                        region_items(on_screen, BLUE, REGION_STROKE_ALPHA, REGION_FILL_ALPHA)
                            .map(clipped),
                    );
                }
            }
        }
        AnnotationAnchor::Element { .. } => {
            if let Some(element) = element_on_canvas(app, draft) {
                let on_screen = frame.screen_rect(element);
                if frame.sees_screen(on_screen) {
                    scene.push(clipped(Item::screen(RectDraw::filled(
                        on_screen,
                        palette::with_alpha(ELEMENT_COLOR, ELEMENT_FILL_ALPHA),
                    ))));
                    scene.push(clipped(Item::screen(PathDraw {
                        commands: super::shape_path::Silhouette::Rect(0.0)
                            .into_path(on_screen.outset(-0.5)),
                        fill: None,
                        stroke: Some(
                            PathStroke::new(
                                palette::with_alpha(ELEMENT_COLOR, ELEMENT_STROKE_ALPHA),
                                1.0,
                            )
                            .dashed(ELEMENT_DASH),
                        ),
                    })));
                }
            }
        }
        // No draft is made on a page point.
        AnnotationAnchor::Page { .. } => {}
    }
}

/// The card, the text in it (or the placeholder) and its selection, caret
/// and input method underline.
pub(crate) fn composer(frame: &Frame<'_>, scene: &mut Scene) {
    let app = frame.app;
    let (Some(draft), Some(card), Some(text_frame), Some(edit)) = (
        app.comment_draft(),
        app.comment_composer(),
        app.edit_frame(),
        app.text_edit().filter(|edit| edit.is_comment()),
    ) else {
        return;
    };
    let on_screen = frame.screen_rect(card);
    let border = Stroke::new(
        palette::with_alpha(frame.colors.composer_border, CARD_BORDER_ALPHA),
        1.0,
        StrokeAlign::Inside,
    );
    scene.push(Item::screen(
        RectDraw::filled(on_screen, frame.colors.composer)
            .with_corner_radius(CARD_RADIUS)
            .with_stroke(border),
    ));
    let key = EntityId::from(draft.id.as_str());
    editing::selection(frame, &key, None, scene);
    if edit.text().is_empty() {
        scene.push(Item::canvas(TextRun::framed(
            PLACEHOLDER,
            &text_frame,
            frame.colors.composer_hint,
        )));
    } else {
        scene.push(Item::canvas(TextRun::framed(
            edit.text(),
            &text_frame,
            frame.colors.composer_ink,
        )));
    }
    editing::caret(frame, &key, None, frame.colors.composer_ink, scene);
}
