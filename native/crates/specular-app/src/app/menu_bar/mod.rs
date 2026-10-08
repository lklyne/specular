//! The native menu bar (macOS).
//!
//! Edit, Arrange, Comment, Tools and View come from `specular_interact::menus`: each item is
//! an `Action` whose shortcut is its row in the binding table. The app menu,
//! File and Window are the shell's own. Choosing an action item sends the
//! action through `update`, the same path its key takes.
//!
//! An enabled item with a Command shortcut takes that key before winit sees
//! it. A disabled one lets the key through, which is how Cmd+C reaches an
//! entered page: the model disables Copy there. So the items are brought in
//! step with the app every turn.

mod keys;

use anyhow::Context as _;
use muda::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use specular_interact::{Action, App, Chord, Event, MenuEntry, menus};

use super::Shell;

/// A menu item that is the shell's business, not an `Action`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShellCommand {
    /// Choose a `.canvas` file and show it in place of this one.
    Open,
    /// Write unsaved changes now instead of when the autosave is due.
    Save,
    /// Close the window, which ends the app.
    Close,
    /// End the app.
    Quit,
}

/// The File menu: label, command and shortcut. `None` is a dividing line.
const FILE_ITEMS: [Option<(&str, ShellCommand, Chord)>; 4] = [
    Some(("Open…", ShellCommand::Open, Chord::char('o').cmd())),
    Some(("Save", ShellCommand::Save, Chord::char('s').cmd())),
    None,
    Some(("Close", ShellCommand::Close, Chord::char('w').cmd())),
];

/// A native item for one `Action`.
enum ActionItem {
    Plain(MenuItem),
    Checked(CheckMenuItem),
}

/// Whether an item can be chosen, and whether it is checked.
type ItemState = (bool, Option<bool>);

/// The installed menu bar.
pub(super) struct MenuBar {
    /// Owns the native menus.
    _menu: Menu,
    /// The action items, in the order `menus` lists them.
    actions: Vec<(MenuId, Action, ActionItem)>,
    shell: Vec<(MenuId, ShellCommand)>,
    /// The states the native items were last given.
    shown: Vec<ItemState>,
}

/// What `menus` says each action item is now, in order.
fn states(app: &App) -> Vec<ItemState> {
    (menus(app).into_iter())
        .flat_map(|menu| menu.entries)
        .filter_map(|entry| match entry {
            MenuEntry::Item(item) => Some((item.enabled, item.checked)),
            MenuEntry::Separator => None,
        })
        .collect()
}

impl MenuBar {
    /// Builds the menus and makes them the application's. Call on the main
    /// thread once the application has launched.
    fn install(app: &App) -> anyhow::Result<Self> {
        let menu = Menu::new();
        let mut shell = Vec::new();
        let mut actions = Vec::new();

        let quit = MenuItem::new(
            "Quit Specular",
            true,
            keys::accelerator(Chord::char('q').cmd()),
        );
        shell.push((quit.id().clone(), ShellCommand::Quit));
        let app_menu = Submenu::new("Specular", true);
        app_menu.append_items(&[
            &PredefinedMenuItem::about(None, None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::services(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;
        menu.append(&app_menu)?;

        let file = Submenu::new("File", true);
        for entry in FILE_ITEMS {
            match entry {
                Some((label, command, chord)) => {
                    let item = MenuItem::new(label, true, keys::accelerator(chord));
                    shell.push((item.id().clone(), command));
                    file.append(&item)?;
                }
                None => file.append(&PredefinedMenuItem::separator())?,
            }
        }
        menu.append(&file)?;

        for model in menus(app) {
            let submenu = Submenu::new(model.title, true);
            for entry in model.entries {
                let MenuEntry::Item(item) = entry else {
                    submenu.append(&PredefinedMenuItem::separator())?;
                    continue;
                };
                let accelerator = item.chord.and_then(keys::accelerator);
                let (id, native) = if let Some(checked) = item.checked {
                    let native = CheckMenuItem::new(item.label, item.enabled, checked, accelerator);
                    submenu.append(&native)?;
                    (native.id().clone(), ActionItem::Checked(native))
                } else {
                    let native = MenuItem::new(item.label, item.enabled, accelerator);
                    submenu.append(&native)?;
                    (native.id().clone(), ActionItem::Plain(native))
                };
                actions.push((id, item.action, native));
            }
            menu.append(&submenu)?;
        }

        let window = Submenu::new("Window", true);
        window.append_items(&[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(None),
        ])?;
        menu.append(&window)?;

        menu.init_for_nsapp();
        window.set_as_windows_menu_for_nsapp();
        Ok(Self {
            _menu: menu,
            actions,
            shell,
            shown: states(app),
        })
    }

    /// The items chosen since the last call, oldest first.
    fn take_chosen(&self) -> Vec<Chosen> {
        let mut chosen = Vec::new();
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            let action = (self.actions.iter())
                .find(|(id, ..)| id == event.id())
                .map(|(_, action, _)| Chosen::Action(action.clone()));
            let command = (self.shell.iter())
                .find(|(id, _)| id == event.id())
                .map(|(_, command)| Chosen::Shell(*command));
            chosen.extend(action.or(command));
        }
        chosen
    }

    /// Enables and checks the items as `app` now has them. `all` writes
    /// every item even if nothing changed: a chosen check item has flipped
    /// its own mark.
    fn refresh(&mut self, app: &App, all: bool) {
        let states = states(app);
        if !all && states == self.shown {
            return;
        }
        for ((_, _, item), (enabled, checked)) in self.actions.iter().zip(&states) {
            match item {
                ActionItem::Plain(item) => item.set_enabled(*enabled),
                ActionItem::Checked(item) => {
                    item.set_enabled(*enabled);
                    item.set_checked(checked.unwrap_or(false));
                }
            }
        }
        self.shown = states;
    }
}

/// A menu item that was chosen.
enum Chosen {
    Action(Action),
    Shell(ShellCommand),
}

impl Shell {
    /// Puts the menu bar up. Without it the app still runs: every action
    /// has its key.
    pub(super) fn install_menu(&mut self) {
        match MenuBar::install(&self.app).context("installing the menu bar") {
            Ok(menu) => self.menu = Some(menu),
            Err(error) => tracing::warn!("{error:#}"),
        }
    }

    /// Runs the items chosen since the last turn, then brings the menu in
    /// step with the app.
    pub(super) fn run_menu(&mut self) {
        let chosen = self.menu.as_ref().map(MenuBar::take_chosen);
        let chosen = chosen.unwrap_or_default();
        let any = !chosen.is_empty();
        for item in chosen {
            match item {
                Chosen::Action(action) => self.dispatch(Event::Action(action)),
                Chosen::Shell(command) => self.run_shell_command(command),
            }
        }
        if let Some(menu) = self.menu.as_mut() {
            menu.refresh(&self.app, any);
        }
    }
}
