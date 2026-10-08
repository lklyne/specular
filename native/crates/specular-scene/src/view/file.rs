//! File entities. A markdown file is a Document. An image file that has
//! loaded is its pixels. Every other file, and an image that is loading,
//! missing or unreadable, is a card with a file glyph and its name.

use specular_doc::{Entity, FileRef, ObjectFit};
use specular_interact::{Image, ImageState};

use super::frame::{Frame, canvas_rect};
use super::{document, image, palette};
use crate::{
    Color, ImageId, Item, PathCommand, PathDraw, PathStroke, Point, RectDraw, Scene, TextAlign,
    TextRun,
};

const CORNER_RADIUS: f32 = 4.0;
const PADDING: f32 = 16.0;
/// The glyph is drawn in a 24-unit box at this scale: 32 canvas units tall.
const GLYPH_SCALE: f32 = 32.0 / 24.0;
const GLYPH_STROKE: f32 = 1.5;
/// Gap between the glyph and the name.
const GAP: f32 = 8.0;
const NAME_SIZE: f32 = 11.0;

pub(crate) fn draw(frame: &Frame<'_>, entity: &Entity, file: &FileRef, scene: &mut Scene) {
    if let Some(note) = frame.app.note(&file.file) {
        document::draw(frame, entity, note, scene);
        return;
    }
    let rect = canvas_rect(entity.rect);
    if let Some(Image { key, state }) = frame.app.image(&file.file) {
        match *state {
            ImageState::Ready { width, height } => {
                // An `<img>` with no `object-fit` set contains.
                let fit = file.object_fit.unwrap_or(ObjectFit::Contain);
                let draw = image::fitted(ImageId(key.0), rect, width, height, fit);
                scene.push(Item::canvas(draw));
                return;
            }
            ImageState::Loading | ImageState::Missing | ImageState::Failed => {}
        }
    }
    scene.push(palette::card_shadow(frame.colors, rect, CORNER_RADIUS));
    scene.push(Item::canvas(
        RectDraw::filled(rect, frame.colors.card).with_corner_radius(CORNER_RADIUS),
    ));
    // The glyph and one line of name, centred together as a column.
    let name_line = NAME_SIZE * TextRun::DEFAULT_LINE_HEIGHT;
    let glyph_size = 24.0 * GLYPH_SCALE;
    let column = glyph_size + GAP + name_line;
    let centre = rect.centre();
    let top = centre.y - column / 2.0;
    let glyph_origin = Point::new(centre.x - glyph_size / 2.0, top);
    scene.push(Item::canvas(glyph(glyph_origin, frame.colors.file_glyph)).clipped(rect));

    let name = file.file.rsplit('/').next().unwrap_or(&file.file);
    let run = TextRun {
        wrap_width: Some((rect.width - PADDING * 2.0).max(0.0)),
        align: TextAlign::Centre,
        ..TextRun::new(
            name,
            Point::new(rect.x + PADDING, top + glyph_size + GAP),
            NAME_SIZE,
            frame.colors.file_glyph,
        )
    };
    scene.push(Item::canvas(run).clipped(rect));
}

/// A sheet of paper with a folded corner, its 24-unit box placed at `origin`.
fn glyph(origin: Point, color: Color) -> PathDraw {
    let at = |x: f32, y: f32| Point::new(origin.x + x * GLYPH_SCALE, origin.y + y * GLYPH_SCALE);
    let line = |x, y| PathCommand::LineTo(at(x, y));
    PathDraw {
        commands: vec![
            PathCommand::MoveTo(at(14.0, 2.0)),
            line(6.0, 2.0),
            line(4.0, 4.0),
            line(4.0, 20.0),
            line(6.0, 22.0),
            line(18.0, 22.0),
            line(20.0, 20.0),
            line(20.0, 8.0),
            PathCommand::Close,
            PathCommand::MoveTo(at(14.0, 2.0)),
            line(14.0, 8.0),
            line(20.0, 8.0),
        ],
        fill: None,
        stroke: Some(PathStroke::new(color, GLYPH_STROKE)),
    }
}
