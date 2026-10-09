//! What the window shows under the chrome: the canvas, or one item alone.
//!
//! An item view is a way of looking at the canvas, not a second mode of it.
//! The item keeps its stored size, nothing is written to the document and
//! no page host changes its viewport (ADR 0020). Three things follow from
//! [`Session::item_view`](crate::Session) and from nothing else:
//!
//! - [`hides`] and [`hides_comment`] take every other item and comment out
//!   of what is seen, which is also what can be hit.
//! - [`settle`] holds the camera on the item, fitted into the free part of
//!   the viewport, and keeps the selection among what is seen.
//! - Leaving puts back the camera the canvas had.

use glam::DVec2;
use specular_core::Camera;
use specular_doc::{Annotation, AnnotationAnchor, Document, Entity, EntityId, ItemId, Kind, Rect};

use crate::anchor::anchors_to_pages;
use crate::app::page_of;
use crate::focus::set_focus;
use crate::notes::{is_note_file, note_file};
use crate::viewport::area;
use crate::{App, Effect, Tool, geometry, update, zoom};

/// What is shown under the chrome.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Showing {
    /// The canvas: every item, where its camera is.
    #[default]
    Canvas,
    /// One page or Document alone, at its stored size, fitted into the
    /// viewport.
    Item(EntityId),
}

/// The item shown alone, and the camera the canvas gets back.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ItemView {
    pub(crate) item: EntityId,
    pub(crate) canvas_camera: Camera,
}

impl App {
    /// What is shown under the chrome.
    pub fn showing(&self) -> Showing {
        self.shown_item()
            .map_or(Showing::Canvas, |item| Showing::Item(item.clone()))
    }

    /// The item shown alone, or `None` while the canvas is shown.
    pub fn shown_item(&self) -> Option<&EntityId> {
        self.session.item_view.as_ref().map(|view| &view.item)
    }

    /// The camera of the canvas itself: the one that is saved, and the one
    /// the canvas comes back with after an item view.
    pub fn canvas_camera(&self) -> Camera {
        (self.session.item_view.as_ref()).map_or(self.session.camera, |view| view.canvas_camera)
    }
}

/// Whether `entity` can be shown alone: a page or a Document.
pub(crate) fn can_show(entity: &Entity) -> bool {
    match &entity.kind {
        Kind::Page(_) => true,
        Kind::File(file) => is_note_file(&file.file),
        Kind::Text(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => false,
    }
}

/// The items that can be shown alone, in an order that holds still: those
/// in `kept` as it lists them, then the rest as the stack has them. The
/// document has no order of its own but the stack, which a reorder changes.
pub(crate) fn listed<'a>(document: &'a Document, kept: &[EntityId]) -> Vec<&'a Entity> {
    let known = (kept.iter())
        .filter_map(|id| document.entity(id))
        .filter(|entity| can_show(entity));
    let new = (document.entities()).filter(|entity| can_show(entity) && !kept.contains(&entity.id));
    known.chain(new).collect()
}

/// Changes what is shown, unless a drag is in flight. An item that cannot
/// be shown alone changes nothing. Showing an item selects it, and a page
/// is entered: its tab is the page, so the wheel and the keys are its own
/// at once. Going back to the canvas leaves the page.
pub(crate) fn show(app: &mut App, showing: Showing, effects: &mut Vec<Effect>) {
    update::verb(app, effects, |app, effects| match showing {
        Showing::Canvas => {
            leave(app);
            set_focus(app, None, effects);
        }
        Showing::Item(item) => {
            let Some(entity) = app.document.entity(&item).filter(|it| can_show(it)) else {
                return;
            };
            let page = page_of(entity).map(|_| item.clone());
            let canvas_camera = app.canvas_camera();
            app.session.selection.set([ItemId::Entity(item.clone())]);
            app.session.item_view = Some(ItemView {
                item,
                canvas_camera,
            });
            set_focus(app, page, effects);
        }
    });
}

/// How wide a Document shown alone is read at, at 100%.
const READING_MEASURE: f64 = 720.0;

/// The rect a Document shown alone is laid out in, in place of its stored
/// one: a reading column of a fixed measure, as tall as the fit leaves
/// room for, so the camera sits on it at 100% and the text scrolls inside.
/// Nothing is written: the stored rect is what the canvas shows. `None` for
/// anything else, a page included, which is never resized.
pub(crate) fn reading_rect(app: &App, entity: &Entity) -> Option<Rect> {
    if !shows(app, &entity.id) || note_file(&entity.kind).is_none() {
        return None;
    }
    let room = zoom::fit_room(area(app).size);
    Some(Rect::new(
        entity.rect.x,
        entity.rect.y,
        READING_MEASURE.min(room.x),
        room.y,
    ))
}

/// Whether `id` is the item shown alone. A press on its body goes into it
/// whatever is selected: there is nothing else it could be picking.
pub(crate) fn shows(app: &App, id: &EntityId) -> bool {
    app.shown_item() == Some(id)
}

/// Goes back to the canvas, with the camera it had.
pub(crate) fn leave(app: &mut App) {
    if let Some(view) = app.session.item_view.take() {
        app.session.camera = view.canvas_camera;
    }
}

/// Brings the session in step with what is shown after an event: the kept
/// order takes in new items and drops gone ones, an item view whose item is
/// gone falls back to the canvas, and one that stands has its camera fitted
/// and its selection kept among what is seen.
pub(crate) fn settle(app: &mut App) {
    keep_order(app);
    let Some(view) = app.session.item_view.clone() else {
        return;
    };
    let Some(entity) = app.document.entity(&view.item).filter(|it| can_show(it)) else {
        leave(app);
        return;
    };
    let free = area(app);
    let rect = reading_rect(app, entity).unwrap_or(entity.rect);
    let mut camera = zoom::fitting(rect, free.size);
    camera.pan += free.min.as_vec2();
    app.session.camera = camera;

    let document = &app.document;
    let seen = |id: &EntityId| document.entity(id).is_some_and(|it| !hides_from(&view, it));
    app.session.selection.retain(|item| match item {
        ItemId::Entity(id) => seen(id),
        ItemId::Edge(id) => document
            .edge(id)
            .is_some_and(|edge| seen(&edge.from) && seen(&edge.to)),
    });
}

fn keep_order(app: &mut App) {
    let document = &app.document;
    let kept = &app.session.shown_order;
    let count = document.entities().filter(|it| can_show(it)).count();
    let stands =
        count == kept.len() && (kept.iter()).all(|id| document.entity(id).is_some_and(can_show));
    if !stands {
        app.session.shown_order = (listed(document, kept).into_iter())
            .map(|entity| entity.id.clone())
            .collect();
    }
}

/// Whether an item view leaves `entity` out: it is neither the item shown
/// nor hooked to it, nor what a drag is making, which is hooked when the
/// drag ends.
pub(crate) fn hides(app: &App, entity: &Entity) -> bool {
    (app.session.item_view.as_ref()).is_some_and(|view| hides_from(view, entity))
        && app.creating() != Some(&entity.id)
}

/// Whether an item view would hide `entity` as soon as it was added, hooked
/// where it can be: anything in a Document's view, and beside a page
/// whatever cannot be hooked to one.
pub(crate) fn hides_new(app: &App, entity: &Entity) -> bool {
    let Some(view) = &app.session.item_view else {
        return false;
    };
    let page = (app.document.entity(&view.item)).is_some_and(|item| page_of(item).is_some());
    !(page && entity.parent.is_none() && anchors_to_pages(&entity.kind))
}

/// Whether a press at the canvas point `world` with `tool` makes nothing.
/// In an item view what is made off the page shown, and any page or
/// Document, would be hidden as soon as it existed.
pub(crate) fn refuses(app: &App, tool: Tool, world: DVec2) -> bool {
    let Some(view) = &app.session.item_view else {
        return false;
    };
    let hooks = match tool {
        Tool::AddText | Tool::AddSticky | Tool::AddShape | Tool::Draw | Tool::Comment => true,
        Tool::AddPage | Tool::AddDocument => false,
        Tool::Select | Tool::Inspect => return false,
    };
    let on_page = (app.page_placement(&view.item))
        .is_some_and(|placement| geometry::contains(placement.rect, world));
    !(hooks && on_page)
}

fn hides_from(view: &ItemView, entity: &Entity) -> bool {
    let hooked = (entity.anchor.as_ref()).is_some_and(|anchor| anchor.page_id == view.item);
    entity.id != view.item && !hooked
}

/// Whether an item view leaves `annotation` out: it is not on the page
/// shown.
pub(crate) fn hides_comment(app: &App, annotation: &Annotation) -> bool {
    let Some(view) = &app.session.item_view else {
        return false;
    };
    let on = match &annotation.anchor {
        AnnotationAnchor::Page { page_id, .. } | AnnotationAnchor::Element { page_id, .. } => {
            Some(page_id)
        }
        AnnotationAnchor::Canvas { .. } | AnnotationAnchor::Region(_) => {
            annotation.page_anchor.as_ref().map(|it| &it.page_id)
        }
    };
    on != Some(&view.item)
}
