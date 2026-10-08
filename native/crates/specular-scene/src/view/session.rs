//! The session layer, drawn over every entity: the hover border, selection
//! outlines, the copies an Option-drag is about to leave, alignment and
//! distribution guides, a line's reorder dots and gap bars, resize handles,
//! the marquee and the comment tool's preview. Apart from those copies it
//! is all in screen space, so it keeps its pixel size at any zoom.

use specular_doc::{EntityId, ItemId};
use specular_interact::{CopyPreview, Corner, HANDLE_SIZE, HandleOwner, OUTLINE_PADDING};

use super::annotations::region_items;
use super::edge_chrome;
use super::frame::Frame;
use super::guides;
use super::layout_handles;
use super::palette;
use super::shape_path::Silhouette;
use crate::{
    Color, Dash, Item, PathDraw, PathStroke, Rect, RectDraw, Scene, Space, Stroke, StrokeAlign,
};

const OUTLINE_WIDTH: f32 = 1.0;
const HANDLE_STROKE: f32 = 1.0;
const DROP_TARGET_WIDTH: f32 = 2.0;
/// How far outside an entered group its dashed ring sits, and how it is
/// dashed.
const ENTERED_GAP: f32 = 4.0;
const ENTERED_DASH: Dash = Dash { on: 5.0, off: 3.0 };
const MARQUEE_FILL_ALPHA: f32 = 0.12;
const MARQUEE_BORDER_ALPHA: f32 = 0.9;
/// The comment tool's region while it is dragged out.
const PREVIEW_COLOR: Color = Color::rgb(0x2b, 0x7f, 0xff);
const PREVIEW_STROKE_ALPHA: f32 = 0.9;
const PREVIEW_FILL_ALPHA: f32 = 0.1;
const COPY_GHOST_OPACITY: f32 = 0.5;

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
        if let Some(entity) = app.document().entity(id)
            && let Some(seen) = specular_interact::shown_rect(app, entity)
        {
            push_outline(frame, seen, scene);
        }
    }

    if let Some(preview) = app.copy_preview() {
        push_copy_ghosts(frame, &preview, scene);
    }

    // The group being worked inside: a dashed ring in the selection colour a
    // few pixels outside it, apart from a selected group's solid outline.
    if let Some(group) = (app.entered_group()).and_then(|id| app.document().entity(id)) {
        let ring = frame.screen_rect(group.rect).outset(ENTERED_GAP);
        if frame.sees_screen(ring) {
            let stroke =
                PathStroke::new(frame.colors.selection, OUTLINE_WIDTH).dashed(ENTERED_DASH);
            scene.push(Item::screen(PathDraw {
                commands: Silhouette::Rect(0.0).into_path(ring),
                fill: None,
                stroke: Some(stroke),
            }));
        }
    }

    // The group a release would drop the dragged items into: a square 2 px
    // ring in the selection colour round it, in place of its thin border.
    if let Some(group) = (app.group_drop_target()).and_then(|id| app.document().entity(id)) {
        let ring = frame.screen_rect(group.rect);
        if frame.sees_screen(ring) {
            let stroke = Stroke::new(
                frame.colors.selection,
                DROP_TARGET_WIDTH,
                StrokeAlign::Outside,
            );
            scene.push(Item::screen(RectDraw::outlined(ring, stroke)));
        }
    }

    guides::draw(frame, scene);
    layout_handles::draw(frame, scene);

    if let Some((owner, bounds)) = app.handles() {
        match owner {
            // The entity's own outline is already there.
            HandleOwner::Entity(_) => {}
            HandleOwner::Selection => push_outline(frame, bounds, scene),
        }
        let outline = frame.screen_rect(bounds).outset(OUTLINE_PADDING);
        scene.extend(Corner::ALL.map(|corner| handle(outline, corner, frame.colors.selection)));
    }

    if let Some(marquee) = app.marquee() {
        let border = Stroke::new(
            palette::with_alpha(frame.colors.selection, MARQUEE_BORDER_ALPHA),
            OUTLINE_WIDTH,
            StrokeAlign::Inside,
        );
        let fill = palette::with_alpha(frame.colors.selection, MARQUEE_FILL_ALPHA);
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
    edge_chrome::draw(frame, scene);
}

/// A one-pixel ring just outside `rect`, so it never covers what it frames.
fn push_outline(frame: &Frame<'_>, rect: specular_doc::Rect, scene: &mut Scene) {
    let outline = frame.screen_rect(rect).outset(OUTLINE_PADDING);
    if frame.sees_screen(outline) {
        let stroke = Stroke::new(frame.colors.selection, OUTLINE_WIDTH, StrokeAlign::Inside);
        scene.push(Item::screen(RectDraw::outlined(outline, stroke)));
    }
}

/// An Option-drag leaves the originals where they are until the release,
/// so each copy is shown where it will land: the entity itself, drawn
/// again at half strength, inside the outline it will have. `view` is a
/// pure function of the entity, so the ghost is the real thing and cannot
/// drift from what the release makes.
fn push_copy_ghosts(frame: &Frame<'_>, preview: &CopyPreview, scene: &mut Scene) {
    let (dx, dy) = (preview.delta.x as f32, preview.delta.y as f32);
    // The copy has no title or border of its own yet.
    let bare = frame.without_chrome();
    for id in &preview.entities {
        let Some(entity) = (frame.app.document().entity(id))
            .and_then(|entity| specular_interact::seen(frame.app, entity))
        else {
            continue;
        };
        let entity = &*entity.entity;
        let landing = entity.rect.translated(preview.delta.x, preview.delta.y);
        let outline = frame.screen_rect(landing).outset(OUTLINE_PADDING);
        if !frame.sees_screen(outline) {
            continue;
        }
        let mut ghost = Scene::new();
        super::draw_entity(&bare, entity, &mut ghost);
        scene.extend(ghost.items.into_iter().map(|item| {
            let by = match item.space {
                Space::Canvas => 1.0,
                Space::Screen => frame.zoom(),
            };
            let opacity = item.opacity * COPY_GHOST_OPACITY;
            item.translated(dx * by, dy * by).with_opacity(opacity)
        }));
        let stroke = Stroke::new(frame.colors.selection, OUTLINE_WIDTH, StrokeAlign::Inside);
        scene.push(Item::screen(RectDraw::outlined(outline, stroke)));
    }
}

/// A resize handle centred on a corner of the outline, where hit-testing
/// looks for it.
fn handle(outline: Rect, corner: Corner, color: Color) -> Item {
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
    let stroke = Stroke::new(color, HANDLE_STROKE, StrokeAlign::Inside);
    Item::screen(RectDraw::filled(square, Color::WHITE).with_stroke(stroke))
}
