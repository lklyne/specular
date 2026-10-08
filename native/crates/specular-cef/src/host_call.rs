//! Making the `CefBrowserHost` call a translated input event stands for.

use cef::{
    BrowserHost, CefString, ImplBrowserHost, KeyEvent as CefKeyEvent, KeyEventType,
    MouseButtonType, MouseEvent,
};
use specular_core::{KeyEventKind, PointerButton};

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

/// Performs one translated call; a field-for-field copy by design.
pub(crate) fn dispatch(host: &BrowserHost, call: &HostCall<'_>) {
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
        } => host.send_key_event(Some(&CefKeyEvent {
            type_: key_event_type(kind),
            modifiers: flags,
            windows_key_code,
            native_key_code,
            character,
            unmodified_character: character,
            ..CefKeyEvent::default()
        })),
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
