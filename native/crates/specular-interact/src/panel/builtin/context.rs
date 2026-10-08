//! The context menu: the list a right press opens at the pointer, laid out
//! as the lists under a dropdown are and kept inside the viewport.

use glam::Vec2;

use super::dropdown;
use super::metrics::{EDGE_MARGIN, INSET};
use super::node::{Panel, PanelRect, Surface};
use super::{Ctx, PopupAnchor, PopupModel};
use crate::panel::{Control, MenuTarget, context_menu};
use crate::{App, Hit, Tool};
use specular_doc::ItemId;

/// The menu a right press on a canvas, an item, or a canvas of the sidebar
/// opened.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenu {
    /// What it is about. It closes when that changes.
    pub target: MenuTarget,
    /// Where the pointer was, which is the menu's corner.
    pub at: Vec2,
}

/// The menu of `open` as the model says it is now, or `None` once its
/// target has changed.
pub(super) fn model(ctx: &Ctx<'_>, open: &ContextMenu) -> Option<PopupModel> {
    context_menu(ctx.app, &open.target, open.at)
}

/// `model` laid out at its point. The corner is the point, pulled back inside
/// the viewport and clear of the sidebar when the menu would run off it.
pub(super) fn layout(ctx: &Ctx<'_>, model: &PopupModel, viewport: Vec2) -> Option<Panel> {
    let PopupAnchor::Point(at) = model.anchor else {
        return None;
    };
    let [Control::Choices(choices)] = model.controls.as_slice() else {
        return None;
    };
    let body = dropdown::body(ctx, &choices.content);
    let size = body.size + Vec2::splat(INSET * 2.0);
    let covered = ctx.left();
    let corner = Vec2::new(
        at.x.min(viewport.x - EDGE_MARGIN - size.x)
            .max(covered + EDGE_MARGIN),
        at.y.min(viewport.y - EDGE_MARGIN - size.y).max(EDGE_MARGIN),
    )
    .round();
    let content = corner + Vec2::splat(INSET);
    Some(Panel {
        surface: Surface::Dropdown,
        rect: PanelRect::new(corner.x, corner.y, size.x, size.y),
        menu: body.is_menu(),
        nodes: (body.nodes.into_iter())
            .map(|node| node.moved(content))
            .collect(),
    })
}

/// Opens the menu for a right press at `screen` that landed on `hit`, after
/// selecting what the press is on: an item that is not selected becomes the
/// selection, one that is keeps it, and empty canvas clears it. Returns
/// whether the press was taken. A press on the entered page belongs to the
/// page, and so does any press while a tool, a drag or a text edit has the
/// pointer.
pub(crate) fn open(app: &mut App, screen: Vec2, hit: &Hit) -> bool {
    app.session.panel.built_in && open_for_press(app, screen, hit)
}

/// [`open`] whether or not the built-in panels are on: the menu is then
/// someone else's to show, or nobody's.
pub(crate) fn open_for_press(app: &mut App, screen: Vec2, hit: &Hit) -> bool {
    let session = &app.session;
    if session.tool != Tool::Select || session.gesture.is_some() || session.editing.is_some() {
        return false;
    }
    let on = match hit {
        Hit::PageContent { page, .. } if session.focus.page() == Some(page) => return false,
        Hit::PageContent { page: entity, .. }
        | Hit::EntityBody { entity }
        | Hit::GroupLabel { group: entity }
        | Hit::GroupBorder { group: entity } => Some(ItemId::Entity(entity.clone())),
        Hit::Edge { edge } => Some(ItemId::Edge(edge.clone())),
        // A handle and an anchor belong to what is already selected.
        Hit::Handle { .. } | Hit::Anchor { .. } | Hit::Layout(_) => None,
        Hit::Empty => {
            app.session.selection.set([]);
            None
        }
        Hit::Comment { .. } | Hit::Panel { .. } => return false,
    };
    if let Some(item) = on
        && !app.session.selection.contains(&item)
    {
        app.session.selection.set([item]);
    }
    let items = app.session.selection.items().to_vec();
    let target = if items.is_empty() {
        MenuTarget::Empty
    } else {
        MenuTarget::Selection(items)
    };
    app.session.pointer = Some(screen);
    app.session.panel.open = None;
    let menu = ContextMenu { target, at: screen };
    app.session.panel.menu = context_menu(app, &menu.target, menu.at).map(|_| menu);
    true
}
