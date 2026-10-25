//! Where an item shown in its tab is drawn: filled, framed or fitted.

use std::borrow::Cow;

use glam::DVec2;
use specular_core::Camera;
use specular_doc::{Entity, EntityId, Page, Rect};

use super::tabs::Lens;
use crate::app::is_page;
use crate::notes::note_file;
use crate::viewport::area;
use crate::{App, geometry, zoom};

/// How wide a Document that fills the view is read at, at 100%.
const READING_MEASURE: f64 = 720.0;

/// The bare strip left around a page that fills the view, in screen pixels.
pub(super) const FILL_MARGIN: f64 = 4.0;

/// The corner radius of a window on macOS 26 and later, in points. The
/// system gives no way to ask for it.
pub(super) const WINDOW_CORNER_RADIUS: f64 = 16.0;

/// The rect `entity` fills the view at, in place of its stored one. A page
/// is everything the chrome leaves free but a [`FILL_MARGIN`] on each side,
/// in whole pixels, so at 100% one of its CSS pixels is one of the screen's. A Document is a reading column of
/// a fixed measure, as tall as the fit leaves room for, so the camera sits
/// on it at 100% and the text scrolls inside.
pub(super) fn fill_rect(app: &App, entity: &Entity) -> Option<Rect> {
    if is_page(entity) {
        let room = area(app).size - DVec2::splat(2.0 * FILL_MARGIN);
        let size = room.round().max(DVec2::ONE);
        return Some(Rect::new(entity.rect.x, entity.rect.y, size.x, size.y));
    }
    note_file(&entity.kind)?;
    let room = zoom::fit_room(area(app).size);
    Some(Rect::new(
        entity.rect.x,
        entity.rect.y,
        READING_MEASURE.min(room.x),
        room.y,
    ))
}

/// The rect the item shown is laid out in when that is not its stored one,
/// which only Fill does. Nothing is written: the stored rect is what the
/// canvas and Device show. `None` for anything else.
pub(crate) fn presented_rect(app: &App, entity: &Entity) -> Option<Rect> {
    let item = app.shown_item().filter(|item| **item == entity.id)?;
    match app.session.tabs.lens(item) {
        Lens::Fill => fill_rect(app, entity),
        Lens::Device => None,
    }
}

/// `entity` as the view lays it out when that is not as it is stored: at
/// its presented rect, and a page with no device frame, which belongs to
/// the size the page is stored at.
pub(crate) fn presented(app: &App, entity: &Entity) -> Option<Entity> {
    let rect = presented_rect(app, entity)?;
    let bare = |page: Cow<'_, Page>| Page {
        metadata: None,
        ..page.into_owned()
    };
    let kind = (entity.kind.page_fields())
        .and_then(|page| entity.kind.with_page_fields(bare(page)))
        .unwrap_or_else(|| entity.kind.clone());
    Some(Entity {
        rect,
        kind,
        ..entity.clone()
    })
}

/// Whether `id` is the item shown. A press on its body then goes into it
/// whatever is selected: there is nothing else it could be picking.
pub(crate) fn holds(app: &App, id: &EntityId) -> bool {
    app.shown_item() == Some(id)
}

/// The stored rect of `entity` with its device frame around it, when it is
/// a page that shows one.
pub(super) fn framed(entity: &Entity) -> Rect {
    let rect = entity.rect;
    (entity.kind.page_fields())
        .and_then(|page| page.shell())
        .map_or(rect, |shell| {
            let insets = shell.insets;
            Rect::new(
                rect.x - insets.left,
                rect.y - insets.top,
                rect.width + insets.left + insets.right,
                rect.height + insets.top + insets.bottom,
            )
        })
}

/// The camera that holds `entity` in the free part of the viewport. A page
/// that fills it sits a [`FILL_MARGIN`] in from each edge at 100%. Anything else is fitted
/// with room around it, a page with its device frame.
pub(super) fn fitted(app: &App, entity: &Entity) -> Camera {
    let free = area(app);
    let presented = presented_rect(app, entity);
    if let Some(rect) = presented.filter(|_| is_page(entity)) {
        let corner = free.min + DVec2::splat(FILL_MARGIN);
        return Camera::new((corner - geometry::origin(rect)).as_vec2(), 1.0);
    }
    let rect = presented.unwrap_or_else(|| framed(entity));
    let mut camera = zoom::fitting(rect, free.size);
    camera.pan += free.min.as_vec2();
    camera
}
