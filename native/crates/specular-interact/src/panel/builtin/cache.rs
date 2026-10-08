//! The built-in layout, kept between reads.
//!
//! Laying the panels out from the models is the expensive part of an event
//! (a long sidebar costs milliseconds), and a pointer move reads the layout
//! several times: the panels' hit test, the canvas hit test, the cursor,
//! the draw. This keeps the last one and hands it out as a shared
//! [`Arc`], so one read costs a build only when something it is made from
//! has changed.
//!
//! A stale layout is a wrong click, so an entry is used only when both of
//! these hold:
//!
//! 1. Its [`Stamp`] equals the stamp of the app now. The stamp holds
//!    everything the layout reads that is cheap to compare, by value:
//!    viewport, camera, the open list and menu, the sidebar's scroll, the
//!    toolbar, popup and context-menu models (which carry the tool, tool
//!    defaults, selection, page state and editing on-states they were made
//!    from), the history revision, the active canvas, the selection and the
//!    sidebar's folds. A change to any of them can never be missed.
//! 2. It has not been forgotten. The sidebar's rows are made from the whole
//!    document, page states and the space's canvases, which are too big to
//!    compare per read, so [`forget`] drops the entry at the end of every
//!    [`update`](crate::update) that may change them (and where the panels
//!    end a field's edit and read again). Only `update` and
//!    `set_text_measure` mutate an `App`, so a read between events sees the
//!    rows as the last event left them, and the first read of the next
//!    event is the layout the previous one ended with. The events that keep the
//!    entry are a pointer move or leave, a wheel, a pinch and a tick with no
//!    drag in flight (and no edit for a tick): none of them writes the
//!    document, the page states or the canvases, and what they do move
//!    (camera, scroll, hover) is in the stamp or patched below.
//!
//! Hover and press change only each control's [`Pointing`]. They stay out of
//! the stamp and are patched into the kept layout instead, so a pointer that
//! moves from one control to the next rebuilds nothing.
//!
//! The kept layout is the one laid out from the models alone; the editing
//! field's caret and text are put over a copy of it by
//! [`field::overlay`](super::field), because they change on every key.

use std::cell::RefCell;
use std::sync::Arc;

use glam::Vec2;
use specular_core::{Camera, PointerEventKind};

use super::super::{ControlId, PopupModel, ToolbarModel, context_menu, popup_for, toolbar};
use super::{ContextMenu, PanelLayout, PanelUi, Pointing};
use crate::{App, Event, Selection};

/// What the kept layout was made from that can be compared cheaply.
#[derive(Debug, Clone, PartialEq)]
struct Stamp {
    viewport: Vec2,
    camera: Camera,
    open: Option<ControlId>,
    menu: Option<ContextMenu>,
    scroll: f32,
    toolbar: ToolbarModel,
    popup: Option<PopupModel>,
    menu_model: Option<PopupModel>,
    revision: u64,
    switches: u64,
    selection: Selection,
    sidebar: crate::SidebarView,
    focused_comment: Option<specular_doc::AnnotationId>,
}

impl Stamp {
    fn of(app: &App) -> Self {
        let ui = &app.session.panel;
        Self {
            viewport: app.session.viewport,
            camera: app.session.camera,
            open: ui.open.clone(),
            menu: ui.menu.clone(),
            scroll: ui.sidebar_scroll,
            toolbar: toolbar(app),
            popup: popup_for(app),
            menu_model: (ui.menu.as_ref())
                .and_then(|open| context_menu(app, &open.target, open.at)),
            revision: app.history.revision(),
            switches: app.space.switches(),
            selection: app.session.selection.clone(),
            sidebar: app.session.sidebar.clone(),
            focused_comment: app.session.focused_comment.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct Entry {
    stamp: Stamp,
    layout: Arc<PanelLayout>,
    hover: Option<ControlId>,
    pressed: Option<ControlId>,
}

/// The layout kept for an [`App`]. Part of [`PanelUi`], so every app has its
/// own and a clone starts with the layout it was cloned with.
#[derive(Debug, Clone, Default)]
pub struct LayoutCache(RefCell<Option<Entry>>);

/// Caches never differ: two apps are equal when what they show is.
impl PartialEq for LayoutCache {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// Drops the kept layout, so the next read lays out again.
pub(crate) fn forget(app: &App) {
    app.session.panel.cache.0.replace(None);
}

/// Drops the kept layout unless the event that just ran was one that
/// [`keeps_layout`].
pub(crate) fn forget_unless(app: &App, keeps: bool) {
    if !keeps {
        forget(app);
    }
}

/// Whether `event` leaves the document, the page states and the canvases
/// alone, so the built-in layout read before it is still the one after. A
/// drag in flight moves the document under any of them, and an edit under a
/// tick.
pub(crate) fn keeps_layout(app: &App, event: &Event) -> bool {
    let idle = app.session.gesture.is_none();
    match event {
        Event::Pointer(input) => {
            idle && matches!(input.kind, PointerEventKind::Move | PointerEventKind::Leave)
        }
        Event::Wheel(_) | Event::Pinch { .. } => idle,
        Event::Tick { .. } => idle && app.session.editing.is_none(),
        Event::Key(_)
        | Event::Ime(_)
        | Event::Page { .. }
        | Event::Image { .. }
        | Event::Note { .. }
        | Event::NoteCreated { .. }
        | Event::NoteHeights(_)
        | Event::ViewportResized(_)
        | Event::DocumentOpened(_)
        | Event::SpaceOpened(_)
        | Event::CanvasFileChanged { .. }
        | Event::Clipboard(_)
        | Event::FilesDropped { .. }
        | Event::ElementAt { .. }
        | Event::RegionGrab { .. }
        | Event::ToolDefaultsLoaded(_)
        | Event::Action(_)
        | Event::BuiltinPanels(_)
        | Event::Api(_) => false,
    }
}

/// The layout of the panels from the models alone, built only when nothing
/// kept matches.
pub(super) fn base(app: &App) -> Arc<PanelLayout> {
    let ui = &app.session.panel;
    if !ui.built_in {
        return Arc::default();
    }
    let stamp = Stamp::of(app);
    {
        let mut slot = ui.cache.0.borrow_mut();
        if let Some(entry) = slot.as_mut().filter(|entry| entry.stamp == stamp) {
            if entry.hover != ui.hover || entry.pressed != ui.pressed {
                repoint(Arc::make_mut(&mut entry.layout), ui);
                entry.hover.clone_from(&ui.hover);
                entry.pressed.clone_from(&ui.pressed);
            }
            return Arc::clone(&entry.layout);
        }
    }
    let layout = Arc::new(super::build(app));
    ui.cache.0.replace(Some(Entry {
        stamp,
        layout: Arc::clone(&layout),
        hover: ui.hover.clone(),
        pressed: ui.pressed.clone(),
    }));
    layout
}

/// Marks the hovered and pressed controls of `layout`, as laying it out
/// with `ui` would.
fn repoint(layout: &mut PanelLayout, ui: &PanelUi) {
    let panels = [
        &mut layout.sidebar,
        &mut layout.sidebar_list,
        &mut layout.toolbar,
        &mut layout.popup,
        &mut layout.dropdown,
    ];
    for node in panels
        .into_iter()
        .flatten()
        .flat_map(|panel| &mut panel.nodes)
    {
        if let Some(id) = &node.id {
            node.state.pointing = Pointing::of(
                node.state.enabled,
                ui.hover.as_ref() == Some(id),
                ui.pressed.as_ref() == Some(id),
            );
        }
    }
}
