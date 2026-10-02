//! Physical key -> CEF key code tables.
//!
//! Keys are named by W3C UI Events `code` strings (`"KeyA"`, `"ArrowLeft"`,
//! `"ShiftLeft"`), which winit's `KeyCode` variant names follow exactly, so
//! the app can pass `format!("{code:?}")` without this crate depending on
//! winit. Chromium keys DOM `keyCode` off `windows_key_code` on every
//! platform; on macOS it also reads `native_key_code` (a `kVK_*` virtual key
//! code) to derive DOM `code` and to drive editing commands.

/// Which of a pair of keys was pressed (CEF `EVENTFLAG_IS_LEFT` / `_RIGHT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyLocation {
    /// Not one of a left/right pair.
    Standard,
    /// The left key of a pair (`ShiftLeft`).
    Left,
    /// The right key of a pair (`ShiftRight`).
    Right,
}

/// Key codes CEF's `cef_key_event_t` needs for one physical key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCodes {
    /// Windows virtual-key code (`VK_*`), CEF `windows_key_code`.
    pub windows: i32,
    /// macOS virtual key code (`kVK_*`), CEF `native_key_code` on macOS.
    pub mac: i32,
    /// Left/right location for modifier pairs.
    pub location: KeyLocation,
}

const fn key(windows: i32, mac: i32) -> KeyCodes {
    KeyCodes {
        windows,
        mac,
        location: KeyLocation::Standard,
    }
}

const fn left(windows: i32, mac: i32) -> KeyCodes {
    KeyCodes {
        windows,
        mac,
        location: KeyLocation::Left,
    }
}

const fn right(windows: i32, mac: i32) -> KeyCodes {
    KeyCodes {
        windows,
        mac,
        location: KeyLocation::Right,
    }
}

/// `(code, codes)`; Windows values from `WinUser.h`, macOS values from
/// `HIToolbox/Events.h`. Modifier pairs use the generic `VK_SHIFT` /
/// `VK_CONTROL` / `VK_MENU` (what DOM `keyCode` reports) plus a location.
const TABLE: &[(&str, KeyCodes)] = &[
    ("KeyA", key(0x41, 0x00)),
    ("KeyB", key(0x42, 0x0B)),
    ("KeyC", key(0x43, 0x08)),
    ("KeyD", key(0x44, 0x02)),
    ("KeyE", key(0x45, 0x0E)),
    ("KeyF", key(0x46, 0x03)),
    ("KeyG", key(0x47, 0x05)),
    ("KeyH", key(0x48, 0x04)),
    ("KeyI", key(0x49, 0x22)),
    ("KeyJ", key(0x4A, 0x26)),
    ("KeyK", key(0x4B, 0x28)),
    ("KeyL", key(0x4C, 0x25)),
    ("KeyM", key(0x4D, 0x2E)),
    ("KeyN", key(0x4E, 0x2D)),
    ("KeyO", key(0x4F, 0x1F)),
    ("KeyP", key(0x50, 0x23)),
    ("KeyQ", key(0x51, 0x0C)),
    ("KeyR", key(0x52, 0x0F)),
    ("KeyS", key(0x53, 0x01)),
    ("KeyT", key(0x54, 0x11)),
    ("KeyU", key(0x55, 0x20)),
    ("KeyV", key(0x56, 0x09)),
    ("KeyW", key(0x57, 0x0D)),
    ("KeyX", key(0x58, 0x07)),
    ("KeyY", key(0x59, 0x10)),
    ("KeyZ", key(0x5A, 0x06)),
    ("Digit0", key(0x30, 0x1D)),
    ("Digit1", key(0x31, 0x12)),
    ("Digit2", key(0x32, 0x13)),
    ("Digit3", key(0x33, 0x14)),
    ("Digit4", key(0x34, 0x15)),
    ("Digit5", key(0x35, 0x17)),
    ("Digit6", key(0x36, 0x16)),
    ("Digit7", key(0x37, 0x1A)),
    ("Digit8", key(0x38, 0x1C)),
    ("Digit9", key(0x39, 0x19)),
    ("Minus", key(0xBD, 0x1B)),
    ("Equal", key(0xBB, 0x18)),
    ("BracketLeft", key(0xDB, 0x21)),
    ("BracketRight", key(0xDD, 0x1E)),
    ("Backslash", key(0xDC, 0x2A)),
    ("Semicolon", key(0xBA, 0x29)),
    ("Quote", key(0xDE, 0x27)),
    ("Backquote", key(0xC0, 0x32)),
    ("Comma", key(0xBC, 0x2B)),
    ("Period", key(0xBE, 0x2F)),
    ("Slash", key(0xBF, 0x2C)),
    ("Enter", key(0x0D, 0x24)),
    ("Tab", key(0x09, 0x30)),
    ("Space", key(0x20, 0x31)),
    ("Backspace", key(0x08, 0x33)),
    ("Escape", key(0x1B, 0x35)),
    ("Delete", key(0x2E, 0x75)),
    ("Insert", key(0x2D, 0x72)),
    ("Home", key(0x24, 0x73)),
    ("End", key(0x23, 0x77)),
    ("PageUp", key(0x21, 0x74)),
    ("PageDown", key(0x22, 0x79)),
    ("ArrowLeft", key(0x25, 0x7B)),
    ("ArrowRight", key(0x27, 0x7C)),
    ("ArrowDown", key(0x28, 0x7D)),
    ("ArrowUp", key(0x26, 0x7E)),
    ("CapsLock", key(0x14, 0x39)),
    ("ShiftLeft", left(0x10, 0x38)),
    ("ShiftRight", right(0x10, 0x3C)),
    ("ControlLeft", left(0x11, 0x3B)),
    ("ControlRight", right(0x11, 0x3E)),
    ("AltLeft", left(0x12, 0x3A)),
    ("AltRight", right(0x12, 0x3D)),
    ("MetaLeft", left(0x5B, 0x37)),
    ("MetaRight", right(0x5C, 0x36)),
    ("ContextMenu", key(0x5D, 0x6E)),
    ("F1", key(0x70, 0x7A)),
    ("F2", key(0x71, 0x78)),
    ("F3", key(0x72, 0x63)),
    ("F4", key(0x73, 0x76)),
    ("F5", key(0x74, 0x60)),
    ("F6", key(0x75, 0x61)),
    ("F7", key(0x76, 0x62)),
    ("F8", key(0x77, 0x64)),
    ("F9", key(0x78, 0x65)),
    ("F10", key(0x79, 0x6D)),
    ("F11", key(0x7A, 0x67)),
    ("F12", key(0x7B, 0x6F)),
];

/// CEF key codes for a W3C `code` string, or `None` for keys the spike does
/// not map (numpad, media keys, IME keys). Unmapped keys can still type
/// through `KEYEVENT_CHAR` and IME events.
pub fn key_codes(code: &str) -> Option<KeyCodes> {
    TABLE
        .iter()
        .find_map(|&(name, codes)| (name == code).then_some(codes))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn letter_maps_to_ascii_uppercase_vk() {
        assert_eq!(key_codes("KeyQ").map(|c| c.windows), Some(i32::from(b'Q')));
    }

    #[test]
    fn arrow_left_is_not_a_left_located_key() {
        assert_eq!(
            key_codes("ArrowLeft").map(|c| c.location),
            Some(KeyLocation::Standard)
        );
    }

    #[test]
    fn right_shift_carries_right_location_and_generic_vk() {
        assert_eq!(key_codes("ShiftRight"), Some(right(0x10, 0x3C)));
    }

    #[test]
    fn unknown_code_is_unmapped() {
        assert_eq!(key_codes("NumpadEnter"), None);
    }

    #[test]
    fn code_names_are_unique() {
        let names: HashSet<_> = TABLE.iter().map(|(name, _)| name).collect();
        assert_eq!(names.len(), TABLE.len());
    }

    #[test]
    fn mac_key_codes_are_unique() {
        let codes: HashSet<_> = TABLE.iter().map(|(_, c)| c.mac).collect();
        assert_eq!(codes.len(), TABLE.len());
    }
}
