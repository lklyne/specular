//! Text entities: plain text, and sticky notes.
//!
//! Where the text sits and how it is set comes from the editor's
//! [`TextFrame`](specular_interact::TextFrame), so the caret is measured on
//! the run that is drawn. While the entity is edited the run is the working
//! text, with the selection behind it and the caret over it.

use specular_doc::{ColorPreset, Entity, Text, TextStyle};

use super::editing;
use super::frame::{Frame, canvas_rect};
use super::palette::{self, Palette, Role};
use crate::{Item, RectDraw, Scene, TextRun};

/// A sticky note with no colour is yellow.
const STICKY_DEFAULT: specular_doc::Color = specular_doc::Color::Preset(ColorPreset::Yellow);
/// What a plain text with nothing in it shows, faded.
const PLACEHOLDER: &str = "Add text";
const PLACEHOLDER_ALPHA: f32 = 0.4;

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, text: &Text, scene: &mut Scene) {
    let Some(text_frame) = frame.app.text_frame(&entity.id) else {
        return;
    };
    let id = &entity.id;
    let rect = canvas_rect(entity.rect);
    let shown = frame.app.editing_text(id).unwrap_or(&text.text);
    match text.resolved_style() {
        TextStyle::Plain => {
            let color = palette::resolve_or_neutral(text.color.as_ref(), Palette::Vivid, Role::Ink);
            editing::selection(frame, id, None, scene);
            if shown.is_empty() {
                let faded = palette::with_alpha(color, PLACEHOLDER_ALPHA);
                scene.push(Item::canvas(TextRun::framed(
                    PLACEHOLDER,
                    &text_frame,
                    faded,
                )));
            } else {
                scene.push(Item::canvas(TextRun::framed(shown, &text_frame, color)));
            }
            editing::caret(frame, id, None, color, scene);
        }
        TextStyle::Sticky => {
            let stored = text.color.as_ref().unwrap_or(&STICKY_DEFAULT);
            let fill = palette::resolve(stored, Palette::Soft, Role::Fill);
            scene.push(palette::card_shadow(rect, 0.0));
            scene.push(Item::canvas(RectDraw::filled(rect, fill)));
            editing::selection(frame, id, Some(rect), scene);
            if !shown.is_empty() {
                // Clipped, so the renderer can cull a note without shaping it
                // and long text cannot spill over its neighbours.
                let run = TextRun::framed(shown, &text_frame, palette::INK);
                scene.push(Item::canvas(run).clipped(rect));
            }
            editing::caret(frame, id, Some(rect), palette::INK, scene);
        }
    }
}
