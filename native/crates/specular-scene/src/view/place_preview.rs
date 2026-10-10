//! What the tool in hand would place under the pointer, drawn faded where
//! the click would put it: a page as its blank device, a sticky as its
//! card, plain text as its prompt, a shape as itself and a Document as an
//! empty card.

use specular_doc::{Entity, Kind, Page, Text, TextStyle};
use specular_interact::PlacePreview;

use super::frame::{Frame, canvas_rect};
use super::palette::{self, Palette, Role};
use super::{page, shape, text};
use crate::{Item, Rect, RectDraw, Scene, Stroke, StrokeAlign, TextRun};

const OPACITY: f32 = 0.5;
const BORDER_WIDTH: f32 = 1.0;
/// The corner radius of a Document's card, in canvas units.
const DOCUMENT_RADIUS: f32 = 4.0;

pub(crate) fn draw(frame: &Frame<'_>, scene: &mut Scene) {
    let Some(preview) = frame.app.place_preview() else {
        return;
    };
    let mut ghost = Scene::new();
    match &preview {
        PlacePreview::Document(rect) => {
            card(frame, canvas_rect(*rect), DOCUMENT_RADIUS, &mut ghost);
        }
        PlacePreview::Entity(entity) => match &entity.kind {
            Kind::Page(page) => device(frame, entity, page, &mut ghost),
            Kind::Text(text) => note(frame, &preview, entity, text, &mut ghost),
            Kind::Shape(shape) => shape::draw(frame, entity, shape, &mut ghost),
            Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => {}
        },
    }
    scene.extend(ghost.items.into_iter().map(|item| {
        let opacity = item.opacity * OPACITY;
        item.with_opacity(opacity)
    }));
}

/// A card with a hairline round it that keeps its width at any zoom.
fn card(frame: &Frame<'_>, rect: Rect, radius: f32, scene: &mut Scene) {
    scene.push(Item::canvas(
        RectDraw::filled(rect, frame.colors.card).with_corner_radius(radius),
    ));
    let border = Stroke::new(frame.colors.page_border, BORDER_WIDTH, StrokeAlign::Outside);
    scene.push(Item::screen(
        RectDraw::outlined(frame.project(rect), border).with_corner_radius(radius * frame.zoom()),
    ));
}

/// A page before it has loaded anything: its device frame, when it has one,
/// round a blank screen.
fn device(frame: &Frame<'_>, entity: &Entity, page: &Page, scene: &mut Scene) {
    let screen = canvas_rect(entity.rect);
    let Some(shell) = page.shell() else {
        card(frame, screen, page::CORNER_RADIUS, scene);
        return;
    };
    let outer = page::shell_rect(screen, &shell);
    scene.push(Item::canvas(
        RectDraw::filled(outer, frame.colors.device_bezel)
            .with_corner_radius(shell.corner_radius as f32),
    ));
    card(frame, screen, shell.screen_corner_radius as f32, scene);
}

fn note(
    frame: &Frame<'_>,
    preview: &PlacePreview,
    entity: &Entity,
    text: &Text,
    scene: &mut Scene,
) {
    match text.resolved_style() {
        TextStyle::Plain => {
            let Some(text_frame) = preview.text_frame() else {
                return;
            };
            let color = palette::resolve_or_neutral(
                text.color.as_ref(),
                Palette::Vivid,
                Role::Ink,
                frame.colors,
            );
            scene.push(Item::canvas(TextRun::framed(
                text::PLACEHOLDER,
                &text_frame,
                color,
            )));
        }
        TextStyle::Sticky => {
            let stored = text.color.as_ref().unwrap_or(&text::STICKY_DEFAULT);
            let fill = palette::resolve(stored, Palette::Soft, Role::Fill, frame.colors);
            scene.push(Item::canvas(RectDraw::filled(
                canvas_rect(entity.rect),
                fill,
            )));
        }
    }
}
