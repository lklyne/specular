//! Keys for the canvas, from the raw `NSEvent`.
//!
//! GPUI's key events carry a key name and no key code, so numpad 1 and digit
//! 1 both arrive as `"1"`, and a page needs the code. An app-local monitor
//! sees every key event before GPUI does and notes what it carried. When
//! GPUI then hands the same event to the canvas slot, the slot turns the
//! note into a [`KeyInput`] through the tables the winit shell uses.
//!
//! The monitor only reads. Who gets a key is still GPUI's dispatch: a Kit
//! text field, a key binding, the input method, or the canvas slot.

use std::cell::RefCell;
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use specular_core::Modifiers;
use specular_interact::KeyInput;

use crate::native::{Id, string_of};

/// `NSEventTypeKeyDown`, `KeyUp` and `FlagsChanged`.
const KEY_DOWN: usize = 10;
const KEY_UP: usize = 11;
const FLAGS_CHANGED: usize = 12;

/// `NSEventModifierFlag` bits.
const SHIFT: usize = 1 << 17;
const CONTROL: usize = 1 << 18;
const OPTION: usize = 1 << 19;
const COMMAND: usize = 1 << 20;
const CAPS_LOCK: usize = 1 << 16;

/// What a raw key event is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RawKind {
    /// A key went down.
    Down {
        /// Whether it is an auto-repeat.
        repeat: bool,
    },
    /// A key came up.
    Up,
    /// A modifier key went down or came up.
    Flags,
}

/// What one raw key event carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawKey {
    /// Down, up or a modifier change.
    pub(crate) kind: RawKind,
    /// The virtual key code, `kVK_*`.
    pub(crate) code: u16,
    /// `NSEvent.modifierFlags`.
    pub(crate) flags: usize,
    /// `NSEvent.characters`. Empty for a modifier change.
    pub(crate) characters: String,
}

thread_local! {
    /// The key event `AppKit` is dispatching now.
    static CURRENT: RefCell<Option<RawKey>> = const { RefCell::new(None) };
}

/// The raw key event being dispatched, for the GPUI handler it reached.
pub(crate) fn current() -> Option<RawKey> {
    CURRENT.with(|current| current.borrow().clone())
}

fn read(event: Id) -> Option<RawKey> {
    // SAFETY: `event` is the live `NSEvent` the monitor was called with.
    let kind: usize = unsafe { msg_send![event, type] };
    let kind = match kind {
        KEY_DOWN => RawKind::Down {
            // SAFETY: `isARepeat` is valid for a key-down event.
            repeat: unsafe { msg_send![event, isARepeat] },
        },
        KEY_UP => RawKind::Up,
        FLAGS_CHANGED => RawKind::Flags,
        _ => return None,
    };
    // SAFETY: `keyCode` and `modifierFlags` are valid for all three types.
    let code: u16 = unsafe { msg_send![event, keyCode] };
    // SAFETY: as above.
    let flags: usize = unsafe { msg_send![event, modifierFlags] };
    let characters = if kind == RawKind::Flags {
        String::new()
    } else {
        // SAFETY: `characters` is valid for key-down and key-up events and
        // raises for any other, which the branch above keeps out.
        string_of(unsafe { msg_send![event, characters] })
    };
    Some(RawKey {
        kind,
        code,
        flags,
        characters,
    })
}

/// Installs the monitor. It lives as long as the process.
pub(crate) fn install_monitor() {
    let mask: u64 = (1 << KEY_DOWN) | (1 << KEY_UP) | (1 << FLAGS_CHANGED);
    let block = RcBlock::new(|event: NonNull<AnyObject>| -> Id {
        let event = event.as_ptr();
        let raw = read(event);
        CURRENT.with(|current| *current.borrow_mut() = raw);
        event
    });
    // SAFETY: the block takes and returns an `NSEvent`, as the API asks, and
    // AppKit copies it. The token is dropped on purpose: the monitor is
    // never removed.
    let _: Id = unsafe {
        msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: mask,
            handler: &*block
        ]
    };
}

/// The modifiers in `NSEvent.modifierFlags`.
pub(crate) const fn modifiers(flags: usize) -> Modifiers {
    Modifiers {
        shift: flags & SHIFT != 0,
        control: flags & CONTROL != 0,
        alt: flags & OPTION != 0,
        meta: flags & COMMAND != 0,
    }
}

/// The flag a modifier key sets while it is down.
const fn flag_of(code: u16) -> Option<usize> {
    match code {
        56 | 60 => Some(SHIFT),
        59 | 62 => Some(CONTROL),
        58 | 61 => Some(OPTION),
        54 | 55 => Some(COMMAND),
        57 => Some(CAPS_LOCK),
        _ => None,
    }
}

/// The text a press types, as winit reports it: the characters, with the
/// keys a page expects a character from named, and nothing for a key that
/// types nothing (`AppKit` puts arrows and function keys in a private range).
fn text(raw: &RawKey) -> Option<String> {
    let named = match raw.code {
        36 | 76 => Some("\r"),
        48 => Some("\t"),
        49 => Some(" "),
        51 => Some("\u{8}"),
        53 => Some("\u{1b}"),
        _ => None,
    };
    if let Some(named) = named {
        return Some(named.to_owned());
    }
    let types = !raw.characters.is_empty()
        && (raw.characters.chars())
            .all(|c| !c.is_control() && !('\u{f700}'..='\u{f8ff}').contains(&c));
    types.then(|| raw.characters.clone())
}

/// `raw` as the app's key event.
pub(crate) fn key_input(raw: &RawKey) -> KeyInput {
    let (pressed, repeat) = match raw.kind {
        RawKind::Down { repeat } => (true, repeat),
        RawKind::Up => (false, false),
        RawKind::Flags => (
            flag_of(raw.code).is_some_and(|flag| raw.flags & flag != 0),
            false,
        ),
    };
    let text = (pressed && raw.kind != RawKind::Flags)
        .then(|| text(raw))
        .flatten();
    specular_app::native_key_input(
        u32::from(raw.code),
        pressed,
        repeat,
        text,
        modifiers(raw.flags),
    )
}

#[cfg(test)]
mod tests {
    use specular_interact::Key;

    use super::*;

    fn down(code: u16, flags: usize, characters: &str) -> RawKey {
        RawKey {
            kind: RawKind::Down { repeat: false },
            code,
            flags,
            characters: characters.to_owned(),
        }
    }

    #[test]
    fn a_letter_is_its_physical_key_with_the_text_it_typed() {
        let input = key_input(&down(0, SHIFT, "A"));
        assert_eq!(input.key, Key::Char('a'));
        assert_eq!(input.text.as_deref(), Some("A"));
        assert_eq!((input.windows_key_code, input.native_key_code), (0x41, 0));
        assert!(input.pressed && input.modifiers.shift);
    }

    #[test]
    fn digit_one_and_numpad_one_are_told_apart_by_their_codes() {
        let digit = key_input(&down(18, 0, "1"));
        let numpad = key_input(&down(83, 0, "1"));
        assert_eq!(digit.key, Key::Char('1'));
        assert_eq!((digit.native_key_code, numpad.native_key_code), (18, 83));
        assert_ne!(digit.windows_key_code, numpad.windows_key_code);
    }

    #[test]
    fn return_types_a_carriage_return_and_an_arrow_types_nothing() {
        let enter = key_input(&down(36, 0, "\r"));
        assert_eq!((enter.key, enter.text.as_deref()), (Key::Enter, Some("\r")));
        let left = key_input(&down(123, 0, "\u{f702}"));
        assert_eq!((left.key, left.text), (Key::ArrowLeft, None));
    }

    #[test]
    fn a_modifier_key_is_down_while_its_flag_is_set() {
        let flags = |flags| RawKey {
            kind: RawKind::Flags,
            code: 56,
            flags,
            characters: String::new(),
        };
        assert!(key_input(&flags(SHIFT)).pressed);
        assert!(!key_input(&flags(0)).pressed);
        assert_eq!(key_input(&flags(SHIFT)).text, None);
    }

    #[test]
    fn a_release_carries_no_text() {
        let up = RawKey {
            kind: RawKind::Up,
            ..down(0, 0, "a")
        };
        let input = key_input(&up);
        assert!(!input.pressed);
        assert_eq!(input.text, None);
    }
}
