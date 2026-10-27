//! The menu bar as `AppKit` has it, for a script: what each item shows, and
//! choosing one the way a click on it does.

use std::fmt::Write as _;

use objc2::runtime::Bool;
use objc2::{class, msg_send};

use crate::native::{Id, string_of};

/// `NSEventModifierFlag` bits and the sign macOS draws for each.
const SIGNS: [(usize, char); 4] = [
    (1 << 18, '⌃'),
    (1 << 19, '⌥'),
    (1 << 17, '⇧'),
    (1 << 20, '⌘'),
];

fn main_menu() -> Id {
    // SAFETY: the shared application always exists once GPUI runs.
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        msg_send![app, mainMenu]
    }
}

/// The items of `menu`, after `AppKit` has validated them as it does when
/// the menu opens.
fn items(menu: Id) -> Vec<Id> {
    if menu.is_null() {
        return Vec::new();
    }
    // SAFETY: `menu` is a live `NSMenu`, and its items live as long as it.
    unsafe {
        let _: () = msg_send![menu, update];
        let count: isize = msg_send![menu, numberOfItems];
        (0..count)
            .map(|index| msg_send![menu, itemAtIndex: index])
            .collect()
    }
}

/// One item as a line: its title, a check mark, its key and `(disabled)`.
fn line(item: Id, depth: usize, out: &mut String) {
    // SAFETY: `item` is a live `NSMenuItem`.
    let (separator, title, enabled, state, key, mask, submenu) = unsafe {
        let separator: Bool = msg_send![item, isSeparatorItem];
        let title: Id = msg_send![item, title];
        let enabled: Bool = msg_send![item, isEnabled];
        let state: isize = msg_send![item, state];
        let key: Id = msg_send![item, keyEquivalent];
        let mask: usize = msg_send![item, keyEquivalentModifierMask];
        let submenu: Id = msg_send![item, submenu];
        (
            separator.as_bool(),
            string_of(title),
            enabled.as_bool(),
            state,
            string_of(key),
            mask,
            submenu,
        )
    };
    let indent = "  ".repeat(depth);
    if separator {
        let _ = writeln!(out, "{indent}---");
        return;
    }
    let _ = write!(out, "{indent}{title}");
    if state != 0 {
        out.push_str(" [x]");
    }
    if !key.is_empty() {
        let signs: String = (SIGNS.iter())
            .filter(|(bit, _)| mask & bit != 0)
            .map(|(_, sign)| *sign)
            .collect();
        let _ = write!(out, "  {signs}{}", key.escape_debug());
    }
    if !enabled {
        out.push_str("  (disabled)");
    }
    out.push('\n');
    for child in items(submenu) {
        line(child, depth + 1, out);
    }
}

/// The whole menu bar as text.
pub(super) fn dump() -> String {
    let mut out = String::new();
    for item in items(main_menu()) {
        line(item, 0, &mut out);
    }
    out
}

/// Chooses the item at `path`, titles joined by `>`, as a click on it does.
/// A disabled item is left alone, as `AppKit` leaves it.
pub(super) fn choose(path: &str) {
    let mut menu = main_menu();
    let mut found: Option<(Id, isize)> = None;
    for title in path.split('>').map(str::trim) {
        let at = items(menu).into_iter().position(|item| {
            // SAFETY: `item` is a live `NSMenuItem`.
            let name: Id = unsafe { msg_send![item, title] };
            string_of(name) == title
        });
        let Some(at) = at.and_then(|at| isize::try_from(at).ok()) else {
            tracing::warn!(path, title, "scripted input: no such menu item");
            return;
        };
        found = Some((menu, at));
        // SAFETY: `at` is an index `items` just counted in the live `menu`.
        menu = unsafe {
            let item: Id = msg_send![menu, itemAtIndex: at];
            msg_send![item, submenu]
        };
    }
    let Some((menu, at)) = found else {
        return;
    };
    // SAFETY: as above.
    let enabled = unsafe {
        let item: Id = msg_send![menu, itemAtIndex: at];
        let enabled: Bool = msg_send![item, isEnabled];
        enabled.as_bool()
    };
    tracing::info!(path, enabled, "scripted input: menu item chosen");
    // SAFETY: as above. `AppKit` sends the item's action to its target.
    unsafe {
        let _: () = msg_send![menu, performActionForItemAtIndex: at];
    }
}
