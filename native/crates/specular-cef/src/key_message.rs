//! A key-down with its editing commands, as the devtools message that
//! delivers it.
//!
//! `SendKeyEvent` has no place for commands, and `Input.dispatchKeyEvent`
//! has: the page dispatches the key to its own handlers and, if none takes
//! it, runs the commands as the key's default action. That is the path a
//! browser's own key-down takes on macOS.

use serde_json::json;
use specular_core::EditingKey;

use crate::translate::flags;

/// The fields of a `SendKeyEvent` raw key-down: what the devtools message
/// is built from, and what is sent the plain way if the page refuses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlainKey {
    /// `cef_event_flags_t` bits.
    pub flags: u32,
    /// `windows_key_code`.
    pub windows_key_code: i32,
    /// `native_key_code`.
    pub native_key_code: i32,
    /// `character` and `unmodified_character` (one UTF-16 unit).
    pub character: u16,
}

/// The devtools protocol's modifier bits for CEF's.
fn protocol_modifiers(held: u32) -> u32 {
    [
        (flags::ALT_DOWN, 1),
        (flags::CONTROL_DOWN, 2),
        (flags::COMMAND_DOWN, 4),
        (flags::SHIFT_DOWN, 8),
    ]
    .into_iter()
    .filter_map(|(flag, bit)| (held & flag != 0).then_some(bit))
    .sum()
}

/// The `Input.dispatchKeyEvent` message for the raw key-down `key` carrying
/// `editing`'s commands.
pub fn editing_key_down(id: i32, key: &PlainKey, editing: &EditingKey) -> Vec<u8> {
    json!({
        "id": id,
        "method": "Input.dispatchKeyEvent",
        "params": {
            "type": "rawKeyDown",
            "modifiers": protocol_modifiers(key.flags),
            "windowsVirtualKeyCode": key.windows_key_code,
            "nativeVirtualKeyCode": key.native_key_code,
            "code": editing.code,
            "key": editing.key,
            "commands": editing.commands,
        },
    })
    .to_string()
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    #[test]
    fn option_shift_left_goes_as_a_raw_key_down_with_its_command_and_dom_names() {
        let key = PlainKey {
            flags: flags::ALT_DOWN | flags::SHIFT_DOWN,
            windows_key_code: 0x25,
            native_key_code: 123,
            character: 0xF702,
        };
        let editing = EditingKey {
            commands: vec!["moveWordLeftAndModifySelection".to_owned()],
            code: "ArrowLeft",
            key: "ArrowLeft".to_owned(),
        };
        let message: Value =
            serde_json::from_slice(&editing_key_down(7, &key, &editing)).expect("json");
        assert_eq!(
            message,
            json!({
                "id": 7,
                "method": "Input.dispatchKeyEvent",
                "params": {
                    "type": "rawKeyDown",
                    "modifiers": 9,
                    "windowsVirtualKeyCode": 0x25,
                    "nativeVirtualKeyCode": 123,
                    "code": "ArrowLeft",
                    "key": "ArrowLeft",
                    "commands": ["moveWordLeftAndModifySelection"],
                },
            })
        );
    }
}
