//! The menu bar's Edit, Tools and View menus, as data.
//!
//! Each item is an [`Action`] and takes its shortcut from the row of
//! [`BINDINGS`] that runs the same action, so a menu and the keyboard cannot
//! disagree. The shell turns the model into native menus and sends the
//! action of a chosen item back as an [`Event::Action`](crate::Event).

use specular_doc::ItemId;

use crate::{Action, App, BINDINGS, Binding, Chord, Context, Tool};

/// One menu of the menu bar.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    /// The title in the menu bar.
    pub title: &'static str,
    /// The items, top to bottom.
    pub entries: Vec<MenuEntry>,
}

/// One row of a menu.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuEntry {
    /// Something to choose.
    Item(MenuItem),
    /// A dividing line.
    Separator,
}

/// A menu item that runs an [`Action`].
#[derive(Debug, Clone, PartialEq)]
pub struct MenuItem {
    /// The text.
    pub label: &'static str,
    /// What choosing it does.
    pub action: Action,
    /// The key that does the same, from the binding table.
    pub chord: Option<Chord>,
    /// Whether it can be chosen now.
    pub enabled: bool,
    /// For an item with a check mark, whether it is checked.
    pub checked: Option<bool>,
}

/// The first row of the binding table that runs `action`.
pub fn binding_of(action: &Action) -> Option<&'static Binding> {
    BINDINGS.iter().find(|binding| binding.action == *action)
}

/// The Edit, Tools and View menus for `app` as it is now. The entries and
/// their order never change, only `enabled` and `checked`.
pub fn menus(app: &App) -> Vec<Menu> {
    let item = |label, action| MenuEntry::Item(item(app, label, action));
    let edit = vec![
        item("Undo", Action::Undo),
        item("Redo", Action::Redo),
        MenuEntry::Separator,
        item("Cut", Action::Cut),
        item("Copy", Action::Copy),
        item("Paste", Action::Paste),
        item("Duplicate", Action::Duplicate),
        item("Delete", Action::Delete),
        MenuEntry::Separator,
        item("Select all", Action::SelectAll),
    ];
    let tools = Tool::ALL
        .into_iter()
        .map(|tool| {
            let mut item = self::item(app, tool_label(tool), tool_action(tool));
            item.checked = Some(app.session.tool == tool);
            MenuEntry::Item(item)
        })
        .collect();
    let view = vec![
        item("Zoom in", Action::ZoomIn),
        item("Zoom out", Action::ZoomOut),
        item("Zoom to 100%", Action::ZoomReset),
        item("Zoom to fit", Action::ZoomToFit),
    ];
    vec![
        Menu {
            title: "Edit",
            entries: edit,
        },
        Menu {
            title: "Tools",
            entries: tools,
        },
        Menu {
            title: "View",
            entries: view,
        },
    ]
}

fn item(app: &App, label: &'static str, action: Action) -> MenuItem {
    let binding = binding_of(&action);
    // An item with no key works where the plain canvas keys do.
    let context = binding.map_or(Context::Canvas, |binding| binding.context);
    MenuItem {
        label,
        chord: binding.map(|binding| binding.chord),
        enabled: context.holds(app) && app.session.gesture.is_none() && has_target(app, &action),
        checked: None,
        action,
    }
}

/// Whether `action` has something to act on.
fn has_target(app: &App, action: &Action) -> bool {
    let selection = &app.session.selection;
    match action {
        Action::Undo => app.session.editing.is_some() || app.can_undo(),
        Action::Redo => app.session.editing.is_some() || app.can_redo(),
        Action::Cut | Action::Copy | Action::Duplicate => {
            (selection.items().iter()).any(|item| matches!(item, ItemId::Entity(_)))
        }
        Action::Delete => !selection.is_empty(),
        Action::SelectAll | Action::ZoomToFit => app.document.entities().next().is_some(),
        Action::Cancel
        | Action::SetTool(_)
        | Action::SetToolDefault(_)
        | Action::SetToolVariant(_)
        | Action::Select(_)
        | Action::SetCamera(_)
        | Action::Nudge { .. }
        | Action::Paste
        | Action::ZoomIn
        | Action::ZoomOut
        | Action::ZoomReset => true,
        Action::Format(_) => app.session.editing.is_some(),
    }
}

/// What a tool's menu item runs: the action its key runs, so the two cannot
/// differ, or a plain switch for a tool with no key.
fn tool_action(tool: Tool) -> Action {
    let bound = BINDINGS.iter().find(|binding| match &binding.action {
        Action::SetTool(bound) => *bound == tool,
        Action::SetToolVariant(patch) => patch.tool() == tool,
        _ => false,
    });
    bound.map_or(Action::SetTool(tool), |binding| binding.action.clone())
}

const fn tool_label(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => "Select",
        Tool::AddPage => "Page",
        Tool::AddText => "Text",
        Tool::AddSticky => "Sticky note",
        Tool::AddDocument => "Document",
        Tool::AddShape => "Shape",
        Tool::Draw => "Draw",
        Tool::Comment => "Comment",
    }
}
