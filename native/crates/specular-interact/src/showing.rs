//! What the window shows under the chrome: the canvas, or one item in a
//! tab of its own.
//!
//! An item view is a way of looking at the canvas, not a second mode of it.
//! Nothing is written to the document (ADR 0020, ADR 0045). The view state
//! is three values: the item
//! ([`Session::item_view`](crate::Session)), the [`Lens`] its tab looks
//! through, and the eye, which says whether anything but the item is drawn.
//! Everything else follows from them, here and nowhere else:
//!
//! - [`hides`] and [`hides_comment`] take items and comments out of what is
//!   seen, which is also what can be hit. [`refuses`] and [`hides_new`] keep
//!   anything from being made that would be hidden at once.
//! - [`presented_rect`] is where the item is laid out when that is not its
//!   stored rect. A page's host is laid out at that size for as long as it
//!   is, through `App::page_placement` and `pages::snapshot`.
//! - [`settle`] holds the camera on the item, or leaves it to the tab in the
//!   Canvas lens, and keeps the selection among what is seen.
//! - Leaving puts back the camera the canvas had.

mod gates;
mod tabs;

use glam::DVec2;
use specular_core::Camera;
use specular_doc::{Document, Entity, EntityId, ItemId, Kind, Page, Rect};

pub(crate) use self::gates::{
    comment_out_of_reach, hides, hides_comment, hides_new, only_page, out_of_reach, refuses,
};
pub use self::tabs::Lens;
pub(crate) use self::tabs::Tabs;
use crate::app::page_of;
use crate::focus::set_focus;
use crate::notes::{is_note_file, note_file};
use crate::panel::PAGE_URL;
use crate::viewport::area;
use crate::{App, ControlId, Effect, edit, geometry, live, place, update, zoom};

/// What is shown under the chrome.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Showing {
    /// The canvas: every item, where its camera is.
    #[default]
    Canvas,
    /// One page or Document, through the lens of its tab.
    Item(EntityId),
}

/// The item a tab shows, and the camera the canvas gets back.
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

    /// The item a tab is showing, or `None` while the canvas is shown.
    pub fn shown_item(&self) -> Option<&EntityId> {
        self.session.item_view.as_ref().map(|view| &view.item)
    }

    /// The lens of the tab showing, or `None` on the Canvas tab, which has
    /// no item to look at.
    pub fn lens(&self) -> Option<Lens> {
        self.shown_item().map(|item| self.session.tabs.lens(item))
    }

    /// Whether the eye is open: an item view draws more than its item. It
    /// is one choice for every tab.
    pub fn shows_others(&self) -> bool {
        !self.session.others_hidden
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
/// be shown alone changes nothing. Showing an item selects it. Going back
/// to the canvas leaves the page.
pub(crate) fn show(app: &mut App, showing: Showing, effects: &mut Vec<Effect>) {
    update::verb(app, effects, |app, effects| match showing {
        Showing::Canvas => {
            leave(app);
            set_focus(app, None, effects);
        }
        Showing::Item(item) => show_item(app, &item, effects),
    });
}

/// Shows `item` through the lens its tab keeps. Where the lens holds the
/// camera a page is entered: its tab is the page, so the wheel and the keys
/// are its own at once. The Canvas lens is the canvas, where a page is
/// entered by a second click, and its camera is where the tab left it, or
/// fitted to the item the first time.
fn show_item(app: &mut App, item: &EntityId, effects: &mut Vec<Effect>) {
    let Some(entity) = app.document.entity(item).filter(|it| can_show(it)) else {
        return;
    };
    let page = page_of(entity).map(|_| item.clone());
    let canvas_camera = app.canvas_camera();
    app.session.selection.set([ItemId::Entity(item.clone())]);
    app.session.item_view = Some(ItemView {
        item: item.clone(),
        canvas_camera,
    });
    if held(app).is_some() {
        set_focus(app, page, effects);
        return;
    }
    set_focus(app, None, effects);
    let kept = app.session.tabs.camera(item);
    if let Some(camera) = kept.or_else(|| Some(fitted(app, app.document.entity(item)?))) {
        app.session.camera = camera;
    }
}

/// Changes the lens of the tab showing. The Canvas tab has none.
pub(crate) fn set_lens(app: &mut App, lens: Lens, effects: &mut Vec<Effect>) {
    update::verb(app, effects, |app, effects| {
        if let Some(item) = app.shown_item().cloned() {
            app.session.tabs.set_lens(&item, lens);
            show_item(app, &item, effects);
        }
    });
}

/// Opens or shuts the eye. A page that shutting it hides is left.
pub(crate) fn set_others(app: &mut App, shown: bool, effects: &mut Vec<Effect>) {
    update::verb(app, effects, |app, effects| {
        app.session.others_hidden = !shown;
        let hidden = (app.session.focus.page())
            .and_then(|page| app.document.entity(page))
            .is_some_and(|page| hides(app, page));
        if hidden {
            set_focus(app, None, effects);
        }
    });
}

/// Shows the tab after the one showing, or the one before it, going round
/// at the ends. The canvas is the first tab.
pub(crate) fn step(app: &mut App, forward: bool, effects: &mut Vec<Effect>) {
    let items = listed(&app.document, &app.session.tabs.order);
    let tabs: Vec<Showing> = std::iter::once(Showing::Canvas)
        .chain((items.iter()).map(|entity| Showing::Item(entity.id.clone())))
        .collect();
    let now = app.showing();
    let at = tabs.iter().position(|tab| *tab == now).unwrap_or(0);
    let by = if forward { 1 } else { tabs.len() - 1 };
    let next = tabs[(at + by) % tabs.len()].clone();
    show(app, next, effects);
}

/// A new tab: a page in a free spot of the canvas, shown alone, with the
/// caret in its address. Making the page is the one undo step, and undoing
/// it takes the item away, which puts the canvas back.
pub(crate) fn new_tab(app: &mut App, effects: &mut Vec<Effect>) {
    update::verb(app, effects, |app, effects| {
        let page = place::page_in_free_spot(app);
        let id = page.id.clone();
        live::create(app, page, effects);
        show_item(app, &id, effects);
        edit::focus_field(app, &ControlId::new(PAGE_URL), effects);
    });
}

/// How wide a Document that fills the view is read at, at 100%.
const READING_MEASURE: f64 = 720.0;

/// The rect `entity` fills the view at, in place of its stored one. A page
/// is everything the chrome leaves free, in whole pixels, so at 100% one of
/// its CSS pixels is one of the screen's. A Document is a reading column of
/// a fixed measure, as tall as the fit leaves room for, so the camera sits
/// on it at 100% and the text scrolls inside.
fn fill_rect(app: &App, entity: &Entity) -> Option<Rect> {
    if page_of(entity).is_some() {
        let size = area(app).size.round().max(DVec2::ONE);
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
/// canvas, Device and the Canvas lens show. `None` for anything else.
pub(crate) fn presented_rect(app: &App, entity: &Entity) -> Option<Rect> {
    let item = app.shown_item().filter(|item| **item == entity.id)?;
    match app.session.tabs.lens(item) {
        Lens::Fill => fill_rect(app, entity),
        Lens::Device | Lens::Canvas => None,
    }
}

/// `entity` as the view lays it out when that is not as it is stored: at
/// its presented rect, and a page with no device frame, which belongs to
/// the size the page is stored at.
pub(crate) fn presented(app: &App, entity: &Entity) -> Option<Entity> {
    let rect = presented_rect(app, entity)?;
    let kind = match &entity.kind {
        Kind::Page(page) => Kind::Page(Page {
            metadata: None,
            ..page.clone()
        }),
        kind @ (Kind::Text(_)
        | Kind::File(_)
        | Kind::Group(_)
        | Kind::Drawing(_)
        | Kind::Shape(_)) => kind.clone(),
    };
    Some(Entity {
        rect,
        kind,
        ..entity.clone()
    })
}

/// The item view, while its lens holds the camera on the item.
fn held(app: &App) -> Option<&ItemView> {
    let view = app.session.item_view.as_ref()?;
    match app.session.tabs.lens(&view.item) {
        Lens::Fill | Lens::Device => Some(view),
        Lens::Canvas => None,
    }
}

/// Whether `id` is the item shown and its tab holds it still. A press on
/// its body then goes into it whatever is selected: there is nothing else
/// it could be picking. In the Canvas lens it is pressed as on the canvas.
pub(crate) fn holds(app: &App, id: &EntityId) -> bool {
    held(app).is_some_and(|view| view.item == *id)
}

/// The stored rect of `entity` with its device frame around it, when it is
/// a page that shows one.
fn framed(entity: &Entity) -> Rect {
    let rect = entity.rect;
    page_of(entity).and_then(Page::shell).map_or(rect, |shell| {
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
/// that fills it sits corner to corner at 100%. Anything else is fitted
/// with room around it, a page with its device frame.
fn fitted(app: &App, entity: &Entity) -> Camera {
    let free = area(app);
    let presented = presented_rect(app, entity);
    if let Some(rect) = presented.filter(|_| page_of(entity).is_some()) {
        return Camera::new((free.min - geometry::origin(rect)).as_vec2(), 1.0);
    }
    let rect = presented.unwrap_or_else(|| framed(entity));
    let mut camera = zoom::fitting(rect, free.size);
    camera.pan += free.min.as_vec2();
    camera
}

/// Goes back to the canvas, with the camera it had.
pub(crate) fn leave(app: &mut App) {
    if let Some(view) = app.session.item_view.take() {
        app.session.camera = view.canvas_camera;
    }
}

/// Brings the session in step with what is shown after an event: the tabs
/// take in new items and drop gone ones, an item view whose item is gone
/// falls back to the canvas, and one that stands has its camera fitted, or
/// kept for its tab in the Canvas lens, and its selection kept among what
/// is seen.
pub(crate) fn settle(app: &mut App) {
    app.session.tabs.settle(&app.document);
    let Some(item) = app.shown_item().cloned() else {
        return;
    };
    let Some(entity) = app.document.entity(&item).filter(|it| can_show(it)) else {
        leave(app);
        return;
    };
    if held(app).is_some() {
        app.session.camera = fitted(app, entity);
    } else {
        app.session.tabs.keep_camera(&item, app.session.camera);
    }

    let mut selection = std::mem::take(&mut app.session.selection);
    let document = &app.document;
    let seen = |id: &EntityId| document.entity(id).is_some_and(|it| !hides(app, it));
    selection.retain(|item| match item {
        ItemId::Entity(id) => seen(id),
        ItemId::Edge(id) => document
            .edge(id)
            .is_some_and(|edge| seen(&edge.from) && seen(&edge.to)),
    });
    app.session.selection = selection;
}
