//! A browser `KeyboardEvent` as the [`KeyInput`] the app reads.

use specular_core::Modifiers;
use specular_interact::{Key, KeyInput, PhysicalKey, mac_key_input};

/// One key transition from its DOM `code` (where the key sits) and `key`
/// (what it typed, or its name). A key the table does not know still types
/// its text into a comment.
pub fn key_input(
    code: &str,
    key: &str,
    pressed: bool,
    repeat: bool,
    modifiers: Modifiers,
) -> KeyInput {
    let mut chars = key.chars();
    let typed = match (chars.next(), chars.next()) {
        (Some(character), None) => Some(character),
        _ => None,
    };
    // Command and Control turn a key into a shortcut, which types nothing.
    let types = pressed && !modifiers.meta && !modifiers.control;
    let text = typed.filter(|_| types).map(String::from);
    match PhysicalKey::from_dom_code(code) {
        Some(physical) => {
            let characters = typed.map(String::from).unwrap_or_default();
            KeyInput {
                text,
                ..mac_key_input(
                    physical.mac_key_code(),
                    pressed,
                    repeat,
                    &characters,
                    modifiers,
                )
            }
        }
        None => KeyInput {
            key: Key::Other,
            pressed,
            repeat,
            text,
            character: typed,
            modifiers,
            windows_key_code: 0,
            native_key_code: 0,
            commands: Vec::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_browser_key_is_the_key_a_mac_sends() {
        let shift = Modifiers {
            shift: true,
            ..Modifiers::default()
        };
        let cmd = Modifiers {
            meta: true,
            ..Modifiers::default()
        };
        let cases = [
            ("KeyA", "a", Modifiers::default(), Key::Char('a'), Some("a")),
            ("KeyA", "A", shift, Key::Char('a'), Some("A")),
            ("KeyZ", "z", cmd, Key::Char('z'), None),
            ("Enter", "Enter", Modifiers::default(), Key::Enter, None),
            ("Escape", "Escape", Modifiers::default(), Key::Escape, None),
            ("Equal", "=", cmd, Key::Char('='), None),
        ];
        for (code, key, modifiers, expected, text) in cases {
            let input = key_input(code, key, true, false, modifiers);
            assert_eq!(
                (input.key, input.text.as_deref()),
                (expected, text),
                "{code} {key}"
            );
        }
    }
}
