//! The menu bar and the shell's own shortcuts.
//!
//! Canvas, Edit, Arrange, Comment, Page, Tools and View are built from
//! `specular_interact::menus`, so an item's shortcut is its row of the
//! binding table and nothing else names a key. The app menu, File and
//! Window are the shell's own.
//!
//! A model item's key is bound under a context no element has. macOS shows
//! it beside the item, and GPUI's keymap never fires it: over the canvas a
//! key goes to `update` as a key, where the binding table decides, exactly
//! as in the winit shell. Elsewhere in the window macOS matches the menu's
//! key equivalent and the item runs.
//!
//! macOS greys an item when GPUI says its action is not available, which
//! GPUI decides by the action's type: one with a handler is available. So
//! an item that is enabled is a [`MenuCommand`], which has a handler, and
//! one that is not is a [`MenuUnavailable`], which has none. macOS then
//! draws it grey and does not match its key.

use std::collections::HashSet;

use gpui_kit::{
    Action, App, Global, KeyBinding, Menu, MenuItem, PathPromptOptions, SystemMenuType, actions,
};
use specular_interact::{CanvasAction, Chord, Event, Key, MenuEntry};

use crate::canvas;
use crate::shell;

/// The context of a model item's binding, which no element is in.
const MENU_ONLY: &str = "SpecularMenuItem";

actions!(
    specular,
    [
        /// Quits, once what is unsaved is written and the pages are closed.
        Quit,
        /// Closes the window, which is the same.
        CloseWindow,
        /// File > Open space…
        OpenSpace,
        /// File > Open…
        OpenCanvas,
        /// File > Save: writes every canvas now.
        Save,
        /// File > Rename canvas…: renames in place in the sidebar.
        RenameCanvas,
        /// Specular > Settings…
        Preferences,
        /// Hides the app.
        Hide,
        /// Minimizes the window.
        Minimize,
        /// Zooms the window.
        ZoomWindow,
        /// A key the Kit's root would take for itself, given to the canvas.
        CanvasKey
    ]
);

/// One item of a model menu, by where it is.
#[derive(Action, Clone, PartialEq, Debug)]
#[action(namespace = specular, no_json)]
pub(crate) struct MenuCommand {
    menu: usize,
    item: usize,
}

/// One item of a model menu that cannot be chosen now. Nothing handles it.
#[derive(Action, Clone, PartialEq, Debug)]
#[action(namespace = specular, no_json)]
pub(crate) struct MenuUnavailable {
    menu: usize,
    item: usize,
}

/// The action of the model item at `(menu, item)`: which of the two types
/// says whether macOS draws it enabled.
fn item_action(menu: usize, item: usize, enabled: bool) -> Box<dyn Action> {
    if enabled {
        Box::new(MenuCommand { menu, item })
    } else {
        Box::new(MenuUnavailable { menu, item })
    }
}

/// What the menu bar last showed, to rebuild it only when it changes.
#[derive(Default)]
struct Shown {
    menus: Vec<specular_interact::Menu>,
    /// The model items whose key is already in the keymap.
    bound: HashSet<(usize, usize, Chord)>,
}

impl Global for Shown {}

/// A chord in GPUI's keystroke syntax, or `None` for a key it has no name
/// for.
fn keystroke(chord: Chord) -> Option<String> {
    let key = match chord.key {
        Key::Char(character) => character.to_string(),
        Key::Escape => "escape".to_owned(),
        Key::Enter => "enter".to_owned(),
        Key::Tab => "tab".to_owned(),
        Key::Backspace => "backspace".to_owned(),
        Key::Delete => "delete".to_owned(),
        Key::Space => "space".to_owned(),
        Key::ArrowLeft => "left".to_owned(),
        Key::ArrowRight => "right".to_owned(),
        Key::ArrowUp => "up".to_owned(),
        Key::ArrowDown => "down".to_owned(),
        Key::Home => "home".to_owned(),
        Key::End => "end".to_owned(),
        Key::PageUp => "pageup".to_owned(),
        Key::PageDown => "pagedown".to_owned(),
        Key::Other => return None,
    };
    let mut text = String::new();
    for (held, name) in [
        (chord.alt, "alt-"),
        (chord.shift, "shift-"),
        (chord.cmd, "cmd-"),
    ] {
        if held {
            text.push_str(name);
        }
    }
    text.push_str(&key);
    Some(text)
}

fn shell_menus() -> (Menu, Menu, Menu) {
    let app = Menu::new("Specular").items([
        MenuItem::action("Settings…", Preferences),
        MenuItem::separator(),
        MenuItem::os_submenu("Services", SystemMenuType::Services),
        MenuItem::separator(),
        MenuItem::action("Hide Specular", Hide),
        MenuItem::separator(),
        MenuItem::action("Quit Specular", Quit),
    ]);
    let file = Menu::new("File").items([
        MenuItem::action("Open space…", OpenSpace),
        MenuItem::action("Open…", OpenCanvas),
        MenuItem::action("Save", Save),
        MenuItem::separator(),
        MenuItem::action("Rename canvas…", RenameCanvas),
        MenuItem::separator(),
        MenuItem::action("Close", CloseWindow),
    ]);
    let window = Menu::new("Window").items([
        MenuItem::action("Minimize", Minimize),
        MenuItem::action("Zoom", ZoomWindow),
    ]);
    (app, file, window)
}

/// Brings the menu bar in step with the models: binds the key of any item
/// not yet bound, and rebuilds the bar when a label, a check mark or an
/// enabled state changed.
pub(crate) fn sync(models: &[specular_interact::Menu], cx: &mut App) {
    if cx.global::<Shown>().menus == models {
        return;
    }
    let mut bindings = Vec::new();
    for (menu, model) in models.iter().enumerate() {
        for (item, entry) in model.entries.iter().enumerate() {
            let MenuEntry::Item(entry) = entry else {
                continue;
            };
            let Some(chord) = entry.chord else {
                continue;
            };
            if cx.global::<Shown>().bound.contains(&(menu, item, chord)) {
                continue;
            }
            if let Some(keys) = keystroke(chord) {
                // Both, so a greyed item still shows its key.
                let command = MenuCommand { menu, item };
                bindings.push(KeyBinding::new(&keys, command, Some(MENU_ONLY)));
                let unavailable = MenuUnavailable { menu, item };
                bindings.push(KeyBinding::new(&keys, unavailable, Some(MENU_ONLY)));
                cx.global_mut::<Shown>().bound.insert((menu, item, chord));
            }
        }
    }
    if !bindings.is_empty() {
        cx.bind_keys(bindings);
    }
    let (app, file, window) = shell_menus();
    let mut bar = vec![app, file];
    for (menu, model) in models.iter().enumerate() {
        let items = model.entries.iter().enumerate().map(|(item, entry)| {
            let MenuEntry::Item(entry) = entry else {
                return MenuItem::separator();
            };
            MenuItem::Action {
                name: entry.label.to_string().into(),
                action: item_action(menu, item, entry.enabled),
                os_action: None,
                checked: entry.checked.unwrap_or(false),
                disabled: !entry.enabled,
            }
        });
        bar.push(Menu::new(model.title).items(items));
    }
    bar.push(window);
    cx.set_menus(bar);
    models.clone_into(&mut cx.global_mut::<Shown>().menus);
}

/// The model item a command names, if it is there and can be chosen now.
fn chosen(command: &MenuCommand) -> Option<specular_interact::Action> {
    let models = canvas::models()?;
    let entry = models.menus.get(command.menu)?.entries.get(command.item)?;
    match entry {
        MenuEntry::Item(item) if item.enabled => Some(item.action.clone()),
        MenuEntry::Item(_) | MenuEntry::Separator => None,
    }
}

/// Whether `action` is one a text field has a meaning of its own for, so
/// that the canvas leaves it alone while a field has the keys.
const fn edits_text(action: &specular_interact::Action) -> bool {
    use specular_interact::Action;
    matches!(
        action,
        Action::Undo
            | Action::Redo
            | Action::Cut
            | Action::Copy
            | Action::Paste
            | Action::SelectAll
            | Action::Delete
            | Action::Duplicate
    )
}

fn choose_canvas(cx: &mut App) {
    let chosen = cx.prompt_for_paths(PathPromptOptions {
        files: true,
        directories: false,
        multiple: false,
        prompt: Some("Open".into()),
    });
    cx.spawn(async move |_| {
        let Ok(Ok(Some(paths))) = chosen.await else {
            return;
        };
        let Some(path) = paths.first() else {
            return;
        };
        canvas::with(|canvas| {
            if let Err(error) = canvas.runtime.open_canvas_file(path) {
                tracing::error!("{error:#}");
            }
            canvas.refresh_models();
        });
    })
    .detach();
}

/// Binds the shell's own keys and installs every action's handler.
pub(crate) fn install(cx: &mut App) {
    cx.set_global(Shown::default());
    cx.bind_keys([
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-shift-o", OpenSpace, None),
        KeyBinding::new("cmd-o", OpenCanvas, None),
        KeyBinding::new("cmd-s", Save, None),
        KeyBinding::new("cmd-,", Preferences, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("tab", CanvasKey, Some("Canvas")),
        KeyBinding::new("shift-tab", CanvasKey, Some("Canvas")),
    ]);
    cx.on_action(|command: &MenuCommand, cx| {
        let Some(action) = chosen(command) else {
            return;
        };
        if !edits_text(&action) {
            canvas::dispatch(Event::Action(action));
            return;
        }
        // A field's own Cmd+Z can still arrive as the menu's item. It must
        // not undo the canvas under the person typing.
        shell::with_view(cx, move |view, window, cx| {
            if !view.typing(window, cx) {
                canvas::dispatch(Event::Action(action));
            }
        });
    });
    cx.on_action(|_: &Quit, cx| shell::begin_exit(cx));
    cx.on_action(|_: &CloseWindow, cx| shell::begin_exit(cx));
    cx.on_action(|_: &OpenSpace, cx| crate::spaces::choose(false, cx));
    cx.on_action(|_: &OpenCanvas, cx| choose_canvas(cx));
    cx.on_action(|_: &Save, _| {
        canvas::with(|canvas| canvas.runtime.flush_files());
    });
    cx.on_action(|_: &RenameCanvas, cx| {
        shell::with_view(cx, |_, window, cx| {
            let rename = specular_interact::Action::Canvas(CanvasAction::BeginRename(None));
            crate::view::run(&rename, window, cx);
        });
    });
    cx.on_action(|_: &Preferences, cx| {
        shell::with_view(cx, |_, window, cx| crate::settings::open(window, cx));
    });
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &Minimize, cx| {
        shell::with_view(cx, |_, window, _| window.minimize_window());
    });
    cx.on_action(|_: &ZoomWindow, cx| {
        shell::with_view(cx, |_, window, _| window.zoom_window());
    });
}

#[cfg(test)]
mod tests {
    use specular_interact::BINDINGS;

    use super::*;

    #[test]
    fn a_chord_is_spelled_as_gpui_parses_it() {
        assert_eq!(
            keystroke(Chord::char('z').cmd().shift()).as_deref(),
            Some("shift-cmd-z")
        );
        assert_eq!(keystroke(Chord::char('-').cmd()).as_deref(), Some("cmd--"));
        assert_eq!(
            keystroke(Chord::key(Key::Backspace)).as_deref(),
            Some("backspace")
        );
        assert_eq!(keystroke(Chord::key(Key::Other)), None);

        // And gpui parses every key of the binding table.
        for binding in BINDINGS {
            let Some(keys) = keystroke(binding.chord) else {
                continue;
            };
            assert!(gpui_kit::Keystroke::parse(&keys).is_ok(), "{keys}");
        }
    }
}
