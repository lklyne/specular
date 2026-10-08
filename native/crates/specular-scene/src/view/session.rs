//! The session layer, drawn over every entity: the hover border, selection
//! outlines, resize handles, the marquee and the comment tool's preview.
//! Everything is in screen space, so it keeps its pixel size at any zoom.

use specular_doc::{EntityId, ItemId};
use specular_interact::{Corner, HANDLE_SIZE, HandleOwner, OUTLINE_PADDING};

use super::annotations::region_items;
use super::frame::Frame;
use super::palette;
use crate::{Color, Item, Rect, RectDraw, Scene, Stroke, StrokeAlign};

const OUTLINE_WIDTH: f32 = 1.0;
const HANDLE_STROKE: f32 = 1.0;
const MARQUEE_FILL_ALPHA: f32 = 0.12;
const MARQUEE_BORDER_ALPHA: f32 = 0.9;
/// The comment tool's region while it is dragged out.
const PREVIEW_COLOR: Color = Color::rgb(0x2b, 0x7f, 0xff);
const PREVIEW_STROKE_ALPHA: f32 = 0.9;
const PREVIEW_FILL_ALPHA: f32 = 0.1;
const COPY_GHOST_FILL_ALPHA: f32 = 0.12;

pub(crate) fn draw(frame: &Frame<'_>, scene: &mut Scene) {
    let app = frame.app;
    let session = app.session();
    let selected = |id: &EntityId| session.selection.contains(&ItemId::Entity(id.clone()));

    // What a marquee would take is outlined as the selection will be.
    let previewed = app.marquee_items();
    let outlined = previewed
        .iter()
        .filter_map(|item| match item {
            ItemId::Entity(id) => Some(id),
            ItemId::Edge(_) => None,
        })
        // A drag has the pointer; what it passes over is not a target.
        .chain(session.hover.as_ref().filter(|_| session.gesture.is_none()))
        .filter(|id| !selected(id))
        .chain(session.selection.entities());
    for id in outlined {
        if let Some(entity) = app.document().entity(id) {
            push_outline(frame, entity.rect, scene);
        }
    }

    // An Option-drag leaves the originals where they are until the release,
    // so the copies are shown as ghosts where they will land.
    for rect in app.copy_preview() {
        push_copy_ghost(frame, rect, scene);
    }

    if let Some((owner, bounds)) = app.handles() {
        match owner {
            // The entity's own outline is already there.
            HandleOwner::Entity(_) => {}
            HandleOwner::Selection => push_outline(frame, bounds, scene),
        }
        let outline = frame.screen_rect(bounds).outset(OUTLINE_PADDING);
        scene.extend(Corner::ALL.map(|corner| handle(outline, corner)));
    }

    if let Some(marquee) = app.marquee() {
        let border = Stroke::new(
            palette::with_alpha(palette::SELECTION, MARQUEE_BORDER_ALPHA),
            OUTLINE_WIDTH,
            StrokeAlign::Inside,
        );
        let fill = palette::with_alpha(palette::SELECTION, MARQUEE_FILL_ALPHA);
        scene.push(Item::screen(
            RectDraw::filled(frame.screen_rect(marquee), fill).with_stroke(border),
        ));
    }
    if let Some(region) = session.comment_preview() {
        scene.extend(region_items(
            frame.screen_rect(region),
            PREVIEW_COLOR,
            PREVIEW_STROKE_ALPHA,
            PREVIEW_FILL_ALPHA,
        ));
    }
}

/// A one-pixel ring just outside `rect`, so it never covers what it frames.
fn push_outline(frame: &Frame<'_>, rect: specular_doc::Rect, scene: &mut Scene) {
    let outline = frame.screen_rect(rect).outset(OUTLINE_PADDING);
    if frame.sees_screen(outline) {
        let stroke = Stroke::new(palette::SELECTION, OUTLINE_WIDTH, StrokeAlign::Inside);
        scene.push(Item::screen(RectDraw::outlined(outline, stroke)));
    }
}

/// The outline a copy will have, tinted so it reads as something coming
/// rather than something selected.
fn push_copy_ghost(frame: &Frame<'_>, rect: specular_doc::Rect, scene: &mut Scene) {
    let outline = frame.screen_rect(rect).outset(OUTLINE_PADDING);
    if frame.sees_screen(outline) {
        let stroke = Stroke::new(palette::SELECTION, OUTLINE_WIDTH, StrokeAlign::Inside);
        let fill = palette::with_alpha(palette::SELECTION, COPY_GHOST_FILL_ALPHA);
        scene.push(Item::screen(
            RectDraw::filled(outline, fill).with_stroke(stroke),
        ));
    }
}

/// A resize handle centred on a corner of the outline, where hit-testing
/// looks for it.
fn handle(outline: Rect, corner: Corner) -> Item {
    let (x, y) = match corner {
        Corner::TopLeft => (outline.x, outline.y),
        Corner::TopRight => (outline.right(), outline.y),
        Corner::BottomRight => (outline.right(), outline.bottom()),
        Corner::BottomLeft => (outline.x, outline.bottom()),
    };
    let square = Rect::new(
        x - HANDLE_SIZE / 2.0,
        y - HANDLE_SIZE / 2.0,
        HANDLE_SIZE,
        HANDLE_SIZE,
    );
    let stroke = Stroke::new(palette::SELECTION, HANDLE_STROKE, StrokeAlign::Inside);
    Item::screen(RectDraw::filled(square, Color::WHITE).with_stroke(stroke))
}
