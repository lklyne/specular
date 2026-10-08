//! What an entity shows while its text is edited: the selection behind the
//! text, and the caret and the input method's underline over it.
//!
//! The selection is in canvas space, like the text. The caret and the
//! underline are in screen space, so they stay at least a pixel wide however
//! far out the camera is.

use specular_doc::EntityId;

use super::frame::{Frame, canvas_rect};
use super::palette;
use crate::{Color, Item, Rect, RectDraw, Scene, TextRun, VerticalAlign};

/// How far below the top of its em box a line of text is underlined, as a
/// fraction of the text size.
const UNDERLINE_DROP: f32 = 1.05;

/// Whether `id` is the entity being edited.
fn is_edited(frame: &Frame<'_>, id: &EntityId) -> bool {
    (frame.app.text_edit()).is_some_and(|edit| edit.entity() == id)
}

/// The one-line text being edited where the editor laid it out, at its
/// pixel size and not cut off at the width of what it names. It is set at
/// the regular weight the editor measures, so the caret lands on the glyphs.
pub(crate) fn edited_line(frame: &Frame<'_>, shown: &str, color: Color) -> Option<Item> {
    let laid_out = frame.app.edit_frame()?;
    let zoom = frame.zoom();
    let spec = specular_interact::TextSpec {
        size: laid_out.spec.size * zoom,
        line_height: laid_out.spec.line_height * zoom,
        ..laid_out.spec
    };
    let run = TextRun {
        box_height: laid_out.box_height.map(|height| height * zoom),
        vertical_align: VerticalAlign::Middle,
        ..TextRun::set(shown, &spec, frame.screen_point(laid_out.origin), color)
    };
    Some(Item::screen(run))
}

/// The selected text of `id`, to go behind its glyphs. `clip` is in canvas
/// space.
pub(crate) fn selection(frame: &Frame<'_>, id: &EntityId, clip: Option<Rect>, scene: &mut Scene) {
    if !is_edited(frame, id) {
        return;
    }
    let fill = palette::TEXT_SELECTION;
    for rect in frame.app.selection_rects() {
        let item = Item::canvas(RectDraw::filled(canvas_rect(rect), fill));
        scene.push(clipped(item, clip));
    }
}

/// The composition underline and the caret of `id`, to go over its glyphs
/// in the text's `color`. `clip` is in canvas space.
pub(crate) fn caret(
    frame: &Frame<'_>,
    id: &EntityId,
    clip: Option<Rect>,
    color: Color,
    scene: &mut Scene,
) {
    if !is_edited(frame, id) {
        return;
    }
    let app = frame.app;
    let clip = clip.map(|clip| frame.project(clip));
    // As thick as a CSS pixel under the camera, and never under one pixel.
    let thickness = frame.zoom().round().max(1.0);
    let size = (app.edit_frame()).map_or(0.0, |text| text.spec.size) * frame.zoom();
    for rect in app.composition_rects() {
        let line = frame.screen_rect(rect);
        let top = line.y + (line.height - size) / 2.0 + size * UNDERLINE_DROP;
        let under = Rect::new(line.x, top.round(), line.width, thickness);
        scene.push(clipped(Item::screen(RectDraw::filled(under, color)), clip));
    }
    let selecting = app
        .text_edit()
        .is_some_and(|edit| !edit.selection().is_empty());
    if selecting || !app.caret_visible() {
        return;
    }
    if let Some(rect) = app.caret_rect() {
        let line = frame.screen_rect(rect);
        let bar = Rect::new(line.x.round(), line.y, thickness, line.height);
        scene.push(clipped(Item::screen(RectDraw::filled(bar, color)), clip));
    }
}

fn clipped(item: Item, clip: Option<Rect>) -> Item {
    match clip {
        Some(clip) => item.clipped(clip),
        None => item,
    }
}
