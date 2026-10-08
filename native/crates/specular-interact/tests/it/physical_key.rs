//! The key tables: what a platform key event is to a binding and to a page.

use specular_core::Modifiers;
use specular_interact::{Key, mac_key_input};

const NONE: Modifiers = Modifiers {
    shift: false,
    control: false,
    alt: false,
    meta: false,
};
const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..NONE
};
const CONTROL: Modifiers = Modifiers {
    control: true,
    ..NONE
};
const CMD: Modifiers = Modifiers { meta: true, ..NONE };

/// The key code, modifiers and characters of an `NSEvent`, then the binding
/// key, the text, and the Windows virtual-key code a page reads `keyCode` from.
type Case = (u16, Modifiers, &'static str, Key, Option<&'static str>, i32);

#[test]
fn a_mac_key_event_is_its_physical_key_with_the_text_it_typed() {
    let cases: &[Case] = &[
        (0, NONE, "a", Key::Char('a'), Some("a"), 0x41),
        (0, SHIFT, "A", Key::Char('a'), Some("A"), 0x41),
        // A shortcut's letter is still its text; Control makes it a control
        // character, which types nothing.
        (6, CMD, "z", Key::Char('z'), Some("z"), 0x5A),
        (0, CONTROL, "\u{1}", Key::Char('a'), None, 0x41),
        // Digit 1 and keypad 1 type the same and are different keys.
        (18, NONE, "1", Key::Char('1'), Some("1"), 0x31),
        (83, NONE, "1", Key::Other, Some("1"), 0),
        (44, NONE, "/", Key::Char('/'), Some("/"), 0xBF),
        (44, SHIFT, "?", Key::Char('/'), Some("?"), 0xBF),
        // The ISO keyboard's extra key is the backquote key.
        (10, NONE, "§", Key::Char('`'), Some("§"), 0xC0),
        (50, NONE, "`", Key::Char('`'), Some("`"), 0xC0),
        // Keys a page expects a character from, whatever AppKit calls it.
        (36, NONE, "\r", Key::Enter, Some("\r"), 0x0D),
        (76, NONE, "\u{3}", Key::Enter, Some("\r"), 0x0D),
        (48, NONE, "\t", Key::Tab, Some("\t"), 0x09),
        (48, SHIFT, "\u{19}", Key::Tab, Some("\t"), 0x09),
        (49, NONE, " ", Key::Space, Some(" "), 0x20),
        (51, NONE, "\u{7f}", Key::Backspace, Some("\u{8}"), 0x08),
        (53, NONE, "\u{1b}", Key::Escape, Some("\u{1b}"), 0x1B),
        // Keys that type nothing: AppKit's private characters are dropped.
        (123, NONE, "\u{f702}", Key::ArrowLeft, None, 0x25),
        (126, NONE, "\u{f700}", Key::ArrowUp, None, 0x26),
        (117, NONE, "\u{f728}", Key::Delete, None, 0x2E),
        (115, NONE, "\u{f729}", Key::Home, None, 0x24),
        (121, NONE, "\u{f72d}", Key::PageDown, None, 0x22),
        (96, NONE, "\u{f708}", Key::Other, None, 0x74),
        // A modifier key, and a key the table does not hold.
        (56, SHIFT, "", Key::Other, None, 0x10),
        (55, CMD, "", Key::Other, None, 0x5B),
        (105, NONE, "\u{f710}", Key::Other, None, 0),
    ];
    for &(code, modifiers, characters, key, text, windows) in cases {
        let down = mac_key_input(code, true, false, characters, modifiers);
        assert_eq!(
            (down.key, down.text.as_deref(), down.windows_key_code),
            (key, text, windows),
            "key code {code} with {characters:?}"
        );
        assert_eq!(down.native_key_code, i32::from(code));
        assert!(down.pressed && !down.repeat && down.modifiers == modifiers);

        let up = mac_key_input(code, false, false, characters, modifiers);
        assert_eq!(
            (up.key, up.text, up.windows_key_code, up.native_key_code),
            (key, None, windows, i32::from(code)),
            "release of key code {code}"
        );
    }
    assert!(mac_key_input(0, true, true, "a", NONE).repeat);
}
