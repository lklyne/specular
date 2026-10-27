//! What the shell passes on from the pointer, the wheel and the keyboard.

use glam::Vec2;
use specular_core::{Modifiers, PointerEventKind};

/// A pointer event at a screen position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerInput {
    /// What the pointer did.
    pub kind: PointerEventKind,
    /// Where, in logical screen pixels. Unused for a leave.
    pub screen: Vec2,
    /// Modifier keys held.
    pub modifiers: Modifiers,
}

/// A scroll at the pointer's last position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelInput {
    /// The scroll in logical pixels. Positive `y` moves content down.
    pub delta: Vec2,
    /// Modifier keys held.
    pub modifiers: Modifiers,
}

/// One key transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    /// Which key, for bindings.
    pub key: Key,
    /// Down or up.
    pub pressed: bool,
    /// Whether this press is an auto-repeat.
    pub repeat: bool,
    /// The text the press produced, if any.
    pub text: Option<String>,
    /// The character the key carries to a page, on its press and on its
    /// release: what it types, or the platform's own character for a key
    /// that types nothing. `None` for a modifier key. A page host tells a
    /// key from a change of modifiers by it, so a key without one is not
    /// pressed and released in the page.
    pub character: Option<char>,
    /// Modifier keys held.
    pub modifiers: Modifiers,
    /// The Windows virtual-key code, which Chromium derives DOM `keyCode`
    /// from on every platform. Only used when the key goes to a page.
    pub windows_key_code: i32,
    /// The platform scan code, which Chromium derives DOM `code` from. Only
    /// used when the key goes to a page.
    pub native_key_code: i32,
    /// The editing commands the platform's key bindings resolve this press
    /// to, as `AppKit` selector names without the colon (`moveWordLeft`).
    /// The shell asks only while a page has the keys, so a user's own
    /// `DefaultKeyBinding.dict` counts. Only used when the key goes to a
    /// page.
    pub commands: Vec<String>,
}

/// The identity of a key for bindings: the physical key, so a binding sits
/// in the same place on every layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Escape.
    Escape,
    /// Return or keypad Enter.
    Enter,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Forward delete.
    Delete,
    /// Home.
    Home,
    /// End.
    End,
    /// Page Up.
    PageUp,
    /// Page Down.
    PageDown,
    /// The space bar.
    Space,
    /// Left arrow.
    ArrowLeft,
    /// Right arrow.
    ArrowRight,
    /// Up arrow.
    ArrowUp,
    /// Down arrow.
    ArrowDown,
    /// A letter, digit or punctuation key, as its unshifted US-layout
    /// character in lower case.
    Char(char),
    /// Any other key. It can still go to a page.
    Other,
}
