//! The menu bar's Canvas, Edit, Arrange, Comment, Page, Tools and View
//! menus, as data.
//!
//! Each item is an [`Action`] and takes its shortcut from the row of
//! [`BINDINGS`] that runs the same action, so a menu and the keyboard cannot
//! disagree. The shell turns the model into native menus and sends the
//! action of a chosen item back as an [`Event::Action`](crate::Event).

use std::borrow::Cow;

use specular_doc::ItemId;

use crate::{
    Action, App, BINDINGS, Binding, CanvasAction, Chord, Context, PageState, SidebarAction, Tool,
    groups, page_state,
};

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
    pub label: Cow<'static, str>,
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

/// The Canvas, Edit, Arrange, Comment, Page, Tools and View menus for `app`
/// as it is now. The Canvas menu ends in the space's canvases, so its
/// entries change with the space. In every other menu the entries and
/// their order never change, only `enabled` and `checked`.
pub fn menus(app: &App) -> Vec<Menu> {
    let item = |label, action| MenuEntry::Item(item(app, label, action));
    let mut canvas = vec![
        item("New canvas", Action::Canvas(CanvasAction::New)),
        item(
            "Duplicate canvas",
            Action::Canvas(CanvasAction::Duplicate(None)),
        ),
        item("Delete canvas", Action::Canvas(CanvasAction::Delete(None))),
        MenuEntry::Separator,
    ];
    canvas.extend(app.space.canvases().iter().map(|canvas| {
        let mut item = self::item(
            app,
            "",
            Action::Canvas(CanvasAction::Switch(canvas.id.clone())),
        );
        item.label = Cow::Owned(canvas.name.clone());
        item.checked = Some(canvas.is_active());
        MenuEntry::Item(item)
    }));
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
    let arrange = vec![
        item("Bring forward", Action::BringForward),
        item("Send backward", Action::SendBackward),
        item("Bring to front", Action::BringToFront),
        item("Send to back", Action::SendToBack),
        MenuEntry::Separator,
        item("Group", Action::Group),
        item("Ungroup", Action::Ungroup),
        item("Auto-layout", Action::AutoLayout),
    ];
    let comment = vec![
        item("Annotate selection", Action::AnnotateSelection),
        item("Resolve comment", Action::ResolveComment(None)),
        item("Delete comment", Action::DeleteComment(None)),
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
        MenuEntry::Separator,
        item("Toggle sidebar", Action::Sidebar(SidebarAction::Toggle)),
    ];
    vec![
        Menu {
            title: "Canvas",
            entries: canvas,
        },
        Menu {
            title: "Edit",
            entries: edit,
        },
        Menu {
            title: "Arrange",
            entries: arrange,
        },
        Menu {
            title: "Comment",
            entries: comment,
        },
        Menu {
            title: "Page",
            entries: page_entries(app),
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

/// The Page menu: a new page in its own tab, then what the page in the dock
/// can be asked.
fn page_entries(app: &App) -> Vec<MenuEntry> {
    vec![
        MenuEntry::Item(item(app, "New page tab", Action::NewPageTab)),
        page_item(app, "Edit address", Action::EditPageUrl),
        MenuEntry::Separator,
        page_item(app, "Back", Action::PageBack),
        page_item(app, "Forward", Action::PageForward),
        page_item(app, "Reload", Action::PageReload),
        page_item(app, "Stop", Action::PageStop),
    ]
}

pub(crate) fn item(app: &App, label: &'static str, action: Action) -> MenuItem {
    let binding = binding_of(&action);
    // An item with no key works where the plain canvas keys do.
    let context = binding.map_or(Context::Canvas, |binding| binding.context);
    MenuItem {
        label: Cow::Borrowed(label),
        chord: binding.map(|binding| binding.chord),
        enabled: context.holds(app) && app.session.gesture.is_none() && has_target(app, &action),
        checked: None,
        action,
    }
}

/// An item of the Page menu. It works on a selected page as well as an
/// entered one, whatever its key's context, and shows a key only where the
/// key does the same in both: the bracket keys restack a selected page.
fn page_item(app: &App, label: &'static str, action: Action) -> MenuEntry {
    MenuEntry::Item(page_menu_item(app, label, action))
}

/// An item of the Page menu as a [`MenuItem`].
pub(crate) fn page_menu_item(app: &App, label: &'static str, action: Action) -> MenuItem {
    let chord = binding_of(&action)
        .filter(|binding| binding.context == Context::PageTarget)
        .map(|binding| binding.chord);
    MenuItem {
        label: Cow::Borrowed(label),
        chord,
        enabled: Context::PageTarget.holds(app)
            && app.session.gesture.is_none()
            && has_target(app, &action),
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
        Action::Delete => !selection.is_empty() || app.session.focused_comment.is_some(),
        Action::ResolveComment(_) | Action::DeleteComment(_) => {
            app.session.focused_comment.is_some()
        }
        Action::BringForward | Action::SendBackward | Action::BringToFront | Action::SendToBack => {
            !selection.is_empty()
        }
        Action::AnnotateSelection | Action::FocusSelection => selection.entities().next().is_some(),
        Action::Arrange(_) | Action::Group => selection.entities().nth(1).is_some(),
        Action::Ungroup | Action::GroupLayout(_) | Action::GroupGap(_) => {
            groups::lone_group(app).is_some()
        }
        Action::AutoLayout => {
            groups::lone_group(app).is_some() || selection.entities().nth(1).is_some()
        }
        Action::SelectAll | Action::ZoomToFit => app.document.entities().next().is_some(),
        // Choosing the canvas already showing is harmless, and its item
        // has to stay enabled to keep its check mark readable.
        Action::Canvas(_)
        | Action::Chat(_)
        | Action::Repo(_)
        | Action::Space(_)
        | Action::Setting(_)
        | Action::Cancel
        | Action::SetTool(_)
        | Action::SetToolDefault(_)
        | Action::SetTheme(_)
        | Action::SetToolVariant(_)
        | Action::Select(_)
        | Action::Reveal { .. }
        | Action::RevealComment(_)
        | Action::Sidebar(_)
        | Action::SetCamera(_)
        | Action::FocusComment(_)
        | Action::Show(_)
        | Action::ShowNext
        | Action::ShowPrevious
        | Action::NewPageTab
        | Action::Nudge { .. }
        | Action::Paste
        | Action::ZoomIn
        | Action::ZoomOut
        | Action::ZoomReset
        | Action::ZoomTo(_) => true,
        Action::SetProperty(property) => property.applies_to(app),
        Action::Format(_) => app.session.editing.is_some(),
        Action::PageBack => page_can(app, |state| state.can_go_back),
        Action::PageForward => page_can(app, |state| state.can_go_forward),
        Action::PageStop => page_can(app, |state| state.loading),
        Action::PageReload | Action::PageNavigate(_) | Action::EditPageUrl => {
            page_state::target(app).is_some()
        }
        Action::ToggleSync => app.selection_synced().is_some(),
    }
}

/// Whether the page a navigation action is for has said `allowed`.
fn page_can(app: &App, allowed: fn(&PageState) -> bool) -> bool {
    page_state::target(app)
        .and_then(|page| app.page_state(page))
        .is_some_and(allowed)
}

/// What a tool's menu item runs: the action its key runs, so the two cannot
/// differ, or a plain switch for a tool with no key.
pub(crate) fn tool_action(tool: Tool) -> Action {
    let bound = BINDINGS.iter().find(|binding| match &binding.action {
        Action::SetTool(bound) => *bound == tool,
        Action::SetToolVariant(patch) => patch.tool() == tool,
        _ => false,
    });
    bound.map_or(Action::SetTool(tool), |binding| binding.action.clone())
}

pub(crate) const fn tool_label(tool: Tool) -> &'static str {
    match tool {
        Tool::Select => "Select",
        Tool::AddPage => "Page",
        Tool::AddText => "Text",
        Tool::AddSticky => "Sticky note",
        Tool::AddDocument => "Document",
        Tool::AddShape => "Shape",
        Tool::Draw => "Draw",
        Tool::Comment => "Comment",
        Tool::Inspect => "Inspect",
    }
}
