//! [`context_menu`]: the menu a right press opens, as a [`ControlsModel`] of
//! choices.
//!
//! It holds the items Electron's menus for a page and for any other canvas
//! item have (`register-canvas-entity-ipc.ts`): back, forward and reload on
//! a page, duplicate, the four stack-order moves and delete, with the labels
//! and keys the bindings give them. The edit and group items round it out
//! to what the menu bar's Edit and Arrange menus offer for the same target.

use glam::Vec2;
use specular_doc::ItemId;

use super::{
    Choices, Control, ControlId, ControlsModel, DropdownOption, DropdownSection, Face, OptionLayout,
};
use crate::menu::{self, MenuItem};
use crate::{Action, App, CanvasAction, CanvasId, Hit, Tool, groups};

/// What a context menu is about. It closes when its target changes.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuTarget {
    /// A canvas in the sidebar.
    Canvas(CanvasId),
    /// The selected items, as they were when the menu opened.
    Selection(Vec<ItemId>),
    /// Empty canvas.
    Empty,
}

/// The menu a right press on a canvas, an item, or a canvas of the sidebar
/// opened.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextMenu {
    /// What it is about. It closes when that changes.
    pub target: MenuTarget,
    /// Where the pointer was, which is the menu's corner.
    pub at: Vec2,
}

/// Opens the menu for a right press at `screen` that landed on `hit`, when
/// the shell draws one. Returns whether the press was taken.
pub(crate) fn open(app: &mut App, screen: Vec2, hit: &Hit) -> bool {
    app.session.panel.shell_chrome && open_for_press(app, screen, hit)
}

/// Opens the menu for a right press at `screen` that landed on `hit`, after
/// selecting what the press is on: an item that is not selected becomes the
/// selection, one that is keeps it, and empty canvas clears it. Returns
/// whether the press was taken. A press on the entered page belongs to the
/// page, and so does any press while a tool, a drag or a text edit has the
/// pointer.
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
        Hit::Comment { .. } => return false,
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
    app.session.panel.menu = context_menu(app, &menu.target).map(|_| menu);
    true
}

/// An item with the name its control has.
type Entry = (ControlId, MenuItem);

/// One section of the menu: items run together, set apart from the next
/// section by a line.
type Section = Vec<Entry>;

/// The name of an item: `menu.` and its label in lower case, joined by
/// dashes.
fn named(item: MenuItem) -> Entry {
    let slug = item.label.to_lowercase().replace(' ', "-");
    (ControlId::new("menu").child(slug), item)
}

fn option((id, item): Entry) -> DropdownOption {
    DropdownOption {
        id,
        label: item.label.clone(),
        face: Face::text(item.label),
        trailing: None,
        fields: Vec::new(),
        chord: item.chord,
        selected: false,
        enabled: item.enabled,
        action: item.action,
    }
}

fn sections(app: &App, target: &MenuTarget) -> Option<Vec<Section>> {
    match target {
        MenuTarget::Canvas(id) => canvas(app, id),
        MenuTarget::Selection(items) => (items.as_slice() == app.session.selection.items())
            .then(|| selection(app))
            .filter(|sections| !sections.is_empty()),
        MenuTarget::Empty => Some(empty(app)),
    }
}

fn canvas(app: &App, id: &CanvasId) -> Option<Vec<Section>> {
    let row = crate::sidebar(app)
        .canvases
        .into_iter()
        .find(|row| row.id == *id)?;
    let item = |part: &str, label: &'static str, action| {
        let item = MenuItem {
            label: label.into(),
            action,
            chord: None,
            enabled: true,
            checked: None,
        };
        (row.control.child("menu").child(part), item)
    };
    Some(vec![vec![
        item(
            "rename",
            "Rename canvas",
            Action::Canvas(CanvasAction::BeginRename(Some(row.id.clone()))),
        ),
        item("delete", "Delete canvas", row.delete.clone()),
    ]])
}

fn empty(app: &App) -> Vec<Section> {
    vec![
        vec![named(menu::item(app, "Paste", Action::Paste))],
        vec![named(menu::item(app, "Select all", Action::SelectAll))],
    ]
}

/// The items for what is selected: the page's history, the clipboard and
/// duplicate for entities, the stack order, grouping and annotating, delete.
fn selection(app: &App) -> Vec<Section> {
    let item = |label, action| named(menu::item(app, label, action));
    let selection = &app.session.selection;
    let entities = selection.entities().count();
    let page = selection
        .single_entity()
        .and_then(|id| app.document.entity(id))
        .is_some_and(crate::app::is_page);
    let mut sections: Vec<Section> = Vec::new();
    if page {
        sections.push(vec![
            named(menu::page_menu_item(app, "Back", Action::PageBack)),
            named(menu::page_menu_item(app, "Forward", Action::PageForward)),
            named(menu::page_menu_item(app, "Reload", Action::PageReload)),
        ]);
    }
    if entities > 0 {
        sections.push(vec![
            item("Cut", Action::Cut),
            item("Copy", Action::Copy),
            item("Paste", Action::Paste),
            item("Duplicate", Action::Duplicate),
        ]);
    }
    sections.push(vec![
        item("Bring forward", Action::BringForward),
        item("Send backward", Action::SendBackward),
        item("Bring to front", Action::BringToFront),
        item("Send to back", Action::SendToBack),
    ]);
    let mut arrange = Vec::new();
    if entities > 1 {
        arrange.push(item("Group", Action::Group));
    }
    if groups::lone_group(app).is_some() {
        arrange.push(item("Ungroup", Action::Ungroup));
    }
    if entities > 0 {
        arrange.push(item("Annotate selection", Action::AnnotateSelection));
    }
    if !arrange.is_empty() {
        sections.push(arrange);
    }
    sections.push(vec![item("Delete", Action::Delete)]);
    sections
}

/// The menu for `target`, or `None` once the
/// target has changed or gone: a selection that is no longer the one the menu
/// was opened on, a canvas that was removed.
pub fn context_menu(app: &App, target: &MenuTarget) -> Option<ControlsModel> {
    let content = sections(app, target)?
        .into_iter()
        .map(|items| DropdownSection::Options {
            layout: OptionLayout::List,
            options: items.into_iter().map(option).collect(),
        })
        .collect();
    let id = ControlId::new("menu");
    Some(ControlsModel {
        controls: vec![Control::Choices(Choices {
            label: "Menu".into(),
            content,
            id,
        })],
    })
}
