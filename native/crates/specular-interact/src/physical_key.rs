//! Keys by where they sit on the keyboard, and what each one is to a
//! binding and to a page. A shell reads the platform's key code off its own
//! event and gets a [`KeyInput`] here, so every shell goes through one
//! table.

use specular_core::Modifiers;

use crate::{Key, KeyInput};

/// Declares [`PhysicalKey`] and its table: each key with its macOS virtual
/// key code (`kVK_*`, which is `NSEvent.keyCode`) and its Windows
/// virtual-key code (`WinUser.h`), which Chromium derives DOM `keyCode`
/// from on every platform.
macro_rules! physical_keys {
    ($($name:ident = $mac:literal, $windows:literal;)*) => {
        /// A key by its place on the keyboard, whatever the layout prints
        /// on it. Named as the DOM `KeyboardEvent.code` values are.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum PhysicalKey {
            $(#[doc = concat!("`", stringify!($name), "`.")] $name,)*
        }

        /// Every key with its macOS and Windows codes.
        const KEYS: &[(PhysicalKey, u16, i32)] =
            &[$((PhysicalKey::$name, $mac, $windows),)*];
    };
}

physical_keys! {
    Backspace = 51, 0x08;
    Tab = 48, 0x09;
    Enter = 36, 0x0D;
    NumpadEnter = 76, 0x0D;
    ShiftLeft = 56, 0x10;
    ShiftRight = 60, 0x10;
    ControlLeft = 59, 0x11;
    ControlRight = 62, 0x11;
    AltLeft = 58, 0x12;
    AltRight = 61, 0x12;
    CapsLock = 57, 0x14;
    Escape = 53, 0x1B;
    Space = 49, 0x20;
    PageUp = 116, 0x21;
    PageDown = 121, 0x22;
    End = 119, 0x23;
    Home = 115, 0x24;
    ArrowLeft = 123, 0x25;
    ArrowUp = 126, 0x26;
    ArrowRight = 124, 0x27;
    ArrowDown = 125, 0x28;
    Insert = 114, 0x2D;
    Delete = 117, 0x2E;
    Digit0 = 29, 0x30;
    Digit1 = 18, 0x31;
    Digit2 = 19, 0x32;
    Digit3 = 20, 0x33;
    Digit4 = 21, 0x34;
    Digit5 = 23, 0x35;
    Digit6 = 22, 0x36;
    Digit7 = 26, 0x37;
    Digit8 = 28, 0x38;
    Digit9 = 25, 0x39;
    KeyA = 0, 0x41;
    KeyB = 11, 0x42;
    KeyC = 8, 0x43;
    KeyD = 2, 0x44;
    KeyE = 14, 0x45;
    KeyF = 3, 0x46;
    KeyG = 5, 0x47;
    KeyH = 4, 0x48;
    KeyI = 34, 0x49;
    KeyJ = 38, 0x4A;
    KeyK = 40, 0x4B;
    KeyL = 37, 0x4C;
    KeyM = 46, 0x4D;
    KeyN = 45, 0x4E;
    KeyO = 31, 0x4F;
    KeyP = 35, 0x50;
    KeyQ = 12, 0x51;
    KeyR = 15, 0x52;
    KeyS = 1, 0x53;
    KeyT = 17, 0x54;
    KeyU = 32, 0x55;
    KeyV = 9, 0x56;
    KeyW = 13, 0x57;
    KeyX = 7, 0x58;
    KeyY = 16, 0x59;
    KeyZ = 6, 0x5A;
    SuperLeft = 55, 0x5B;
    SuperRight = 54, 0x5C;
    F1 = 122, 0x70;
    F2 = 120, 0x71;
    F3 = 99, 0x72;
    F4 = 118, 0x73;
    F5 = 96, 0x74;
    F6 = 97, 0x75;
    F7 = 98, 0x76;
    F8 = 100, 0x77;
    F9 = 101, 0x78;
    F10 = 109, 0x79;
    F11 = 103, 0x7A;
    F12 = 111, 0x7B;
    Semicolon = 41, 0xBA;
    Equal = 24, 0xBB;
    Comma = 43, 0xBC;
    Minus = 27, 0xBD;
    Period = 47, 0xBE;
    Slash = 44, 0xBF;
    Backquote = 50, 0xC0;
    BracketLeft = 33, 0xDB;
    Backslash = 42, 0xDC;
    BracketRight = 30, 0xDD;
    Quote = 39, 0xDE;
}

impl PhysicalKey {
    /// The key with macOS virtual key code `code`, or `None` for a key the
    /// table does not hold (the keypad's digits, F13 and up, media keys).
    pub fn from_mac_key_code(code: u16) -> Option<Self> {
        // An ISO keyboard's extra key (`kVK_ISO_Section`) sits where ANSI
        // has the backquote.
        if code == 10 {
            return Some(Self::Backquote);
        }
        KEYS.iter()
            .find(|&&(_, mac, _)| mac == code)
            .map(|&(key, _, _)| key)
    }

    /// The Windows virtual-key code.
    pub fn windows_key_code(self) -> i32 {
        KEYS.iter()
            .find(|&&(key, _, _)| key == self)
            .map_or(0, |&(_, _, windows)| windows)
    }

    /// The binding identity of the key.
    pub fn key(self) -> Key {
        match self {
            Self::Escape => Key::Escape,
            Self::Enter | Self::NumpadEnter => Key::Enter,
            Self::Tab => Key::Tab,
            Self::Backspace => Key::Backspace,
            Self::Delete => Key::Delete,
            Self::Home => Key::Home,
            Self::End => Key::End,
            Self::PageUp => Key::PageUp,
            Self::PageDown => Key::PageDown,
            Self::Space => Key::Space,
            Self::ArrowLeft => Key::ArrowLeft,
            Self::ArrowRight => Key::ArrowRight,
            Self::ArrowUp => Key::ArrowUp,
            Self::ArrowDown => Key::ArrowDown,
            other => other.us_character().map_or(Key::Other, Key::Char),
        }
    }

    /// The key that `key` names on a US keyboard, for input that starts from
    /// a binding key: a script or a test. `None` for [`Key::Other`].
    pub fn for_key(key: Key) -> Option<Self> {
        if key == Key::Other {
            return None;
        }
        KEYS.iter()
            .map(|&(physical, _, _)| physical)
            .find(|physical| physical.key() == key)
    }

    /// The macOS virtual key code, `kVK_*`.
    pub fn mac_key_code(self) -> u16 {
        KEYS.iter()
            .find(|&&(key, _, _)| key == self)
            .map_or(0, |&(_, mac, _)| mac)
    }

    /// Whether this is a modifier key, whose press and release are a change
    /// of modifiers and carry no character.
    fn is_modifier(self) -> bool {
        matches!(
            self,
            Self::ShiftLeft
                | Self::ShiftRight
                | Self::ControlLeft
                | Self::ControlRight
                | Self::AltLeft
                | Self::AltRight
                | Self::SuperLeft
                | Self::SuperRight
                | Self::CapsLock
        )
    }

    /// The character `AppKit` gives a key that types nothing, which is the
    /// same on every layout: `NSEvent.characters` for it.
    fn function_character(self) -> Option<char> {
        let code = match self {
            Self::Backspace => 0x7F,
            Self::Tab => 0x09,
            Self::Enter => 0x0D,
            Self::NumpadEnter => 0x03,
            Self::Escape => 0x1B,
            Self::ArrowUp => 0xF700,
            Self::ArrowDown => 0xF701,
            Self::ArrowLeft => 0xF702,
            Self::ArrowRight => 0xF703,
            Self::F1 => 0xF704,
            Self::F2 => 0xF705,
            Self::F3 => 0xF706,
            Self::F4 => 0xF707,
            Self::F5 => 0xF708,
            Self::F6 => 0xF709,
            Self::F7 => 0xF70A,
            Self::F8 => 0xF70B,
            Self::F9 => 0xF70C,
            Self::F10 => 0xF70D,
            Self::F11 => 0xF70E,
            Self::F12 => 0xF70F,
            Self::Insert => 0xF746,
            Self::Delete => 0xF728,
            Self::Home => 0xF729,
            Self::End => 0xF72B,
            Self::PageUp => 0xF72C,
            Self::PageDown => 0xF72D,
            _ => return None,
        };
        char::from_u32(code)
    }

    /// The unshifted US-layout character of a letter, digit or punctuation
    /// key, in lower case.
    fn us_character(self) -> Option<char> {
        let punctuation = match self {
            Self::Minus => '-',
            Self::Equal => '=',
            Self::BracketLeft => '[',
            Self::BracketRight => ']',
            Self::Backslash => '\\',
            Self::Semicolon => ';',
            Self::Quote => '\'',
            Self::Comma => ',',
            Self::Period => '.',
            Self::Slash => '/',
            Self::Backquote => '`',
            // Letter and digit keys have their ASCII character as their
            // virtual-key code.
            other => match u8::try_from(other.windows_key_code()) {
                Ok(vk @ (b'A'..=b'Z' | b'0'..=b'9')) => char::from(vk.to_ascii_lowercase()),
                _ => return None,
            },
        };
        Some(punctuation)
    }
}

/// The text a press types, from the characters the platform's event
/// carried: the characters themselves, with the keys a page expects a
/// character from named, and nothing for a key that types nothing (`AppKit`
/// puts arrows and function keys in a private range).
fn typed(physical: Option<PhysicalKey>, characters: &str) -> Option<String> {
    let named = match physical {
        Some(PhysicalKey::Enter | PhysicalKey::NumpadEnter) => Some("\r"),
        Some(PhysicalKey::Tab) => Some("\t"),
        Some(PhysicalKey::Space) => Some(" "),
        Some(PhysicalKey::Backspace) => Some("\u{8}"),
        Some(PhysicalKey::Escape) => Some("\u{1b}"),
        _ => None,
    };
    if let Some(named) = named {
        return Some(named.to_owned());
    }
    let types = !characters.is_empty()
        && (characters.chars()).all(|c| !c.is_control() && !('\u{f700}'..='\u{f8ff}').contains(&c));
    types.then(|| characters.to_owned())
}

/// The character a key's press and release carry to a page. A key that
/// types nothing has its own; any other has the first of the characters the
/// platform's event carried, or its US-layout one when the event had none (a
/// dead key, or a shell that is not told on release). `None` for a modifier
/// key and for a key that is not known and carried nothing.
fn character(physical: Option<PhysicalKey>, characters: &str) -> Option<char> {
    if physical.is_some_and(PhysicalKey::is_modifier) {
        return None;
    }
    let own = physical.and_then(PhysicalKey::function_character);
    let us = || match physical?.key() {
        Key::Char(character) => Some(character),
        Key::Space => Some(' '),
        _ => None,
    };
    own.or_else(|| characters.chars().next()).or_else(us)
}

/// One key transition as a [`KeyInput`], from the macOS virtual key code of
/// the platform's event (`kVK_*`, `NSEvent.keyCode`) and the characters it
/// carried (`NSEvent.characters`), on a press and on a release.
pub fn mac_key_input(
    key_code: u16,
    pressed: bool,
    repeat: bool,
    characters: &str,
    modifiers: Modifiers,
) -> KeyInput {
    let physical = PhysicalKey::from_mac_key_code(key_code);
    KeyInput {
        key: physical.map_or(Key::Other, PhysicalKey::key),
        pressed,
        repeat,
        text: pressed.then(|| typed(physical, characters)).flatten(),
        character: character(physical, characters),
        modifiers,
        windows_key_code: physical.map_or(0, PhysicalKey::windows_key_code),
        native_key_code: i32::from(key_code),
    }
}
