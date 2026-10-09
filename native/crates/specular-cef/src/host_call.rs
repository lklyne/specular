//! Making the `CefBrowserHost` call a translated input event stands for.

use cef::{
    BrowserHost, CefString, ImplBrowser, ImplBrowserHost, ImplFrame, KeyEvent as CefKeyEvent,
    KeyEventType, MouseButtonType, MouseEvent,
};
use specular_core::{KeyEventKind, PageEdit, PointerButton};

use crate::devtools::{Asked, Devtools};
use crate::key_message::{self, PlainKey};
use crate::translate::{CefRange, HostCall};

fn mouse_event(x: i32, y: i32, flags: u32) -> MouseEvent {
    MouseEvent {
        x,
        y,
        modifiers: flags,
    }
}

fn mouse_button(button: PointerButton) -> MouseButtonType {
    match button {
        PointerButton::Left => MouseButtonType::LEFT,
        PointerButton::Middle => MouseButtonType::MIDDLE,
        PointerButton::Right => MouseButtonType::RIGHT,
    }
}

fn key_event_type(kind: KeyEventKind) -> KeyEventType {
    match kind {
        KeyEventKind::RawDown => KeyEventType::RAWKEYDOWN,
        KeyEventKind::Up => KeyEventType::KEYUP,
        KeyEventKind::Char => KeyEventType::CHAR,
    }
}

fn cef_range(range: CefRange) -> cef::Range {
    cef::Range {
        from: range.from,
        to: range.to,
    }
}

/// `SendKeyEvent` for `key`.
pub(crate) fn send_key(host: &BrowserHost, kind: KeyEventKind, key: &PlainKey) {
    host.send_key_event(Some(&CefKeyEvent {
        type_: key_event_type(kind),
        modifiers: key.flags,
        windows_key_code: key.windows_key_code,
        native_key_code: key.native_key_code,
        character: key.character,
        unmodified_character: key.character,
        ..CefKeyEvent::default()
    }));
}

/// Does `edit` to whatever has the focus in the page.
fn edit_focused(host: &BrowserHost, edit: PageEdit) {
    let Some(frame) = host.browser().and_then(|browser| browser.focused_frame()) else {
        return;
    };
    match edit {
        PageEdit::Undo => frame.undo(),
        PageEdit::Redo => frame.redo(),
        PageEdit::Cut => frame.cut(),
        PageEdit::Copy => frame.copy(),
        PageEdit::Paste => frame.paste(),
        PageEdit::PasteAndMatchStyle => frame.paste_and_match_style(),
        PageEdit::SelectAll => frame.select_all(),
    }
}

/// Performs one translated call; a field-for-field copy by design.
pub(crate) fn dispatch(host: &BrowserHost, devtools: &Devtools, call: &HostCall<'_>) {
    match *call {
        HostCall::MouseMove { x, y, flags, leave } => {
            host.send_mouse_move_event(Some(&mouse_event(x, y, flags)), i32::from(leave));
        }
        HostCall::MouseClick {
            x,
            y,
            flags,
            button,
            up,
            click_count,
        } => host.send_mouse_click_event(
            Some(&mouse_event(x, y, flags)),
            mouse_button(button),
            i32::from(up),
            click_count,
        ),
        HostCall::MouseWheel {
            x,
            y,
            flags,
            delta_x,
            delta_y,
        } => host.send_mouse_wheel_event(Some(&mouse_event(x, y, flags)), delta_x, delta_y),
        HostCall::Key {
            kind,
            flags,
            windows_key_code,
            native_key_code,
            character,
        } => send_key(
            host,
            kind,
            &PlainKey {
                flags,
                windows_key_code,
                native_key_code,
                character,
            },
        ),
        HostCall::EditingKey { key, editing } => {
            let sent = devtools.send(host, Asked::Key(key), |id| {
                key_message::editing_key_down(id, &key, editing)
            });
            if !sent {
                send_key(host, KeyEventKind::RawDown, &key);
            }
        }
        HostCall::Edit(edit) => edit_focused(host, edit),
        HostCall::ImeSetComposition {
            text,
            replacement,
            selection,
        } => host.ime_set_composition(
            Some(&CefString::from(text)),
            None,
            Some(&cef_range(replacement)),
            Some(&cef_range(selection)),
        ),
        HostCall::ImeCommit { text, replacement } => {
            host.ime_commit_text(
                Some(&CefString::from(text)),
                Some(&cef_range(replacement)),
                0,
            );
        }
        HostCall::ImeFinish { keep_selection } => {
            host.ime_finish_composing_text(i32::from(keep_selection));
        }
        HostCall::ImeCancel => host.ime_cancel_composition(),
    }
}
