//! The settings dialog: the space folder, what is shown at launch, the
//! repos, the keyboard shortcuts and what the app is built from.
//!
//! [`settings`] is the dialog's whole model, read from the [`App`] like the
//! toolbar's and the sidebar's. The shortcuts are the rows of
//! [`BINDINGS`], so the table cannot name a key the keyboard does not have.

use std::borrow::Cow;

use crate::first_run::SpaceAction;
use crate::menu::{MenuEntry, menus};
use crate::repos::{ReposPane, repos_pane};
use crate::{Action, App, BINDINGS, Context, Effect, Format, SidebarAction, ToolDefaultPatch};

/// What the app keeps between launches that is not in any canvas or tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppSettings {
    /// Whether the left sidebar is shown when the app opens.
    pub show_sidebar: bool,
    /// Whether the right panel is shown when the app opens.
    pub show_chat: bool,
}

/// A change to the [`AppSettings`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingAction {
    /// Show the left sidebar at launch, or do not.
    ShowSidebar(bool),
    /// Show the right panel at launch, or do not.
    ShowChat(bool),
}

/// One part the app is built from, for the About pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutRow {
    /// What it is.
    pub name: String,
    /// Its version.
    pub version: String,
}

/// The open space, for the General pane.
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceRow {
    /// The folder's own name.
    pub name: String,
    /// The folder's path.
    pub path: String,
    /// Shows it in the file manager.
    pub reveal: Action,
}

/// A switch of the General pane.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingToggle {
    /// The control's name.
    pub name: &'static str,
    /// The setting.
    pub label: &'static str,
    /// What it changes, in a sentence.
    pub detail: &'static str,
    /// Whether it is on.
    pub on: bool,
    /// Turns it the other way.
    pub action: Action,
}

/// The General pane.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralPane {
    /// The open space, or `None` when no folder is open.
    pub space: Option<SpaceRow>,
    /// Asks for another folder and opens it. The folder being left is not
    /// touched.
    pub change: Action,
    /// What is shown when the app opens.
    pub toggles: Vec<SettingToggle>,
}

/// One row of the shortcuts table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutRow {
    /// The keys, as macOS writes them.
    pub keys: String,
    /// What they do.
    pub label: Cow<'static, str>,
    /// Where they do it.
    pub place: &'static str,
}

/// The settings dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsModel {
    /// The space folder and the launch defaults.
    pub general: GeneralPane,
    /// The connected repos and the origins bound to them.
    pub repos: ReposPane,
    /// Every key binding, in the table's order.
    pub shortcuts: Vec<ShortcutRow>,
    /// The app's version and those of what it is built from.
    pub about: Vec<AboutRow>,
}

impl App {
    /// What is kept between launches.
    pub fn settings(&self) -> AppSettings {
        self.settings
    }
}

/// The settings dialog for `app` as it is now.
pub fn settings(app: &App) -> SettingsModel {
    let space = app.space.folder().map(|path| SpaceRow {
        name: (path.rsplit('/').find(|part| !part.is_empty()))
            .unwrap_or(path)
            .to_owned(),
        path: path.to_owned(),
        reveal: Action::Space(SpaceAction::Reveal),
    });
    let toggle = |name, label, detail, on, action: fn(bool) -> SettingAction| SettingToggle {
        name,
        label,
        detail,
        on,
        action: Action::Setting(action(!on)),
    };
    SettingsModel {
        general: GeneralPane {
            space,
            change: Action::Space(SpaceAction::Choose { create: true }),
            toggles: vec![
                toggle(
                    "settings.show-sidebar",
                    "Show the sidebar at launch",
                    "The list of canvases, notes and pages on the left.",
                    app.settings.show_sidebar,
                    SettingAction::ShowSidebar,
                ),
                toggle(
                    "settings.show-chat",
                    "Show the right panel at launch",
                    "The agent threads and the composer on the right.",
                    app.settings.show_chat,
                    SettingAction::ShowChat,
                ),
            ],
        },
        repos: repos_pane(app),
        shortcuts: shortcuts(app),
        about: app.about.clone(),
    }
}

fn shortcuts(app: &App) -> Vec<ShortcutRow> {
    let menus = menus(app);
    let in_menu = |action: &Action| {
        menus
            .iter()
            .flat_map(|menu| &menu.entries)
            .find_map(|entry| match entry {
                MenuEntry::Item(item) if item.action == *action => Some(item.label.clone()),
                MenuEntry::Item(_) | MenuEntry::Separator => None,
            })
    };
    (BINDINGS.iter())
        .map(|binding| ShortcutRow {
            keys: binding.chord.text(),
            label: in_menu(&binding.action).unwrap_or_else(|| unlisted(&binding.action)),
            place: match binding.context {
                Context::Always => "Everywhere",
                Context::Canvas => "Canvas",
                Context::CanvasOrEditing => "Canvas and text",
                Context::Editing => "Text",
                Context::EnteredPage => "Inside a page",
                Context::PageTarget => "Page",
            },
        })
        .collect()
}

/// The name of a bound action no menu lists. One with no name here is
/// shown as the action is written, so a new binding is never left out.
fn unlisted(action: &Action) -> Cow<'static, str> {
    let format = |format: &Format| match format {
        Format::Bold => "Bold".into(),
        Format::Italic => "Italic".into(),
        Format::Code => "Code".into(),
        Format::Strike => "Strikethrough".into(),
        Format::Heading(0) => "Body text".into(),
        Format::Heading(level) => format!("Heading {level}").into(),
        Format::BulletList => "Bulleted list".into(),
        Format::NumberedList => "Numbered list".into(),
        Format::TaskList => "Task list".into(),
    };
    match action {
        Action::SetToolVariant(ToolDefaultPatch::ShapeKind(kind)) => {
            format!("{kind:?} shape").into()
        }
        Action::SetToolVariant(ToolDefaultPatch::Brush(brush)) => format!("{brush:?} brush").into(),
        Action::Nudge { dx, dy } => {
            let direction = match (dx.total_cmp(&0.0), dy.total_cmp(&0.0)) {
                (std::cmp::Ordering::Less, _) => "left",
                (std::cmp::Ordering::Greater, _) => "right",
                (_, std::cmp::Ordering::Less) => "up",
                _ => "down",
            };
            format!("Nudge {direction} by {}", dx.abs().max(dy.abs())).into()
        }
        Action::Format(kind) => format(kind),
        Action::Cancel => "Cancel".into(),
        other => format!("{other:?}").into(),
    }
}

/// The settings a launch read from the preferences file: kept, and applied
/// to what is shown.
pub(crate) fn load(app: &mut App, settings: AppSettings) {
    app.settings = settings;
    if app.session.sidebar.shown() != settings.show_sidebar {
        app.session.sidebar.apply(SidebarAction::Toggle);
    }
    if settings.show_chat {
        app.session.chat.show();
    }
}

/// Runs `action`. A setting is the default for the next launch: what is on
/// screen now is left as the user has it.
pub(crate) fn run(app: &mut App, action: SettingAction, effects: &mut Vec<Effect>) {
    let before = app.settings;
    match action {
        SettingAction::ShowSidebar(on) => app.settings.show_sidebar = on,
        SettingAction::ShowChat(on) => app.settings.show_chat = on,
    }
    if app.settings != before {
        effects.push(Effect::SaveSettings(app.settings));
    }
}
