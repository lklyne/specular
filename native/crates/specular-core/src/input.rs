//! Input forwarded into a page.
//!
//! Positions are page-local CSS pixels (CEF view/DIP coordinates): the app
//! converts a screen point with [`Camera::screen_to_world`](crate::Camera::screen_to_world)
//! minus the page's canvas origin, divided by `rect.width / viewport.width`.
//! The shape follows CEF's `SendMouse*Event` / `SendKeyEvent` / `Ime*` calls
//! so the CEF backend is a field-for-field translation.

use std::ops::Range;

use glam::Vec2;

/// Modifier keys and held buttons at the time of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors CEF's EVENTFLAG bitset one flag per field for readability"
)]
pub struct Modifiers {
    /// Shift held.
    pub shift: bool,
    /// Control held.
    pub control: bool,
    /// Option/Alt held.
    pub alt: bool,
    /// Command/Meta held.
    pub meta: bool,
}

/// Mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerButton {
    /// Primary button.
    Left,
    /// Wheel button.
    Middle,
    /// Secondary button.
    Right,
}

/// What a pointer event does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerEventKind {
    /// Pointer moved (buttons may be held — see `buttons`).
    Move,
    /// Button pressed; `click_count` is 1 for single, 2 for double click.
    Down {
        /// Which button.
        button: PointerButton,
        /// Consecutive click count.
        click_count: u8,
    },
    /// Button released.
    Up {
        /// Which button.
        button: PointerButton,
        /// Consecutive click count of the press being released.
        click_count: u8,
    },
    /// Pointer left the page.
    Leave,
}

/// A pointer event at a page-local CSS position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerEvent {
    /// Event kind.
    pub kind: PointerEventKind,
    /// Page-local position in CSS pixels.
    pub position: Vec2,
    /// Modifier keys.
    pub modifiers: Modifiers,
}

/// A wheel/trackpad scroll forwarded into a page (a page-scroll, not a
/// canvas pan — the app decides which before forwarding).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelEvent {
    /// Page-local position in CSS pixels.
    pub position: Vec2,
    /// Scroll delta in CSS pixels; positive `y` scrolls content up (CEF sign).
    pub delta: Vec2,
    /// Modifier keys.
    pub modifiers: Modifiers,
}

/// Key event phase, matching CEF's `cef_key_event_type_t`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventKind {
    /// Key pressed, before any character translation (`KEYEVENT_RAWKEYDOWN`).
    RawDown,
    /// Key released (`KEYEVENT_KEYUP`).
    Up,
    /// A translated character (`KEYEVENT_CHAR`).
    Char,
}

/// A keyboard event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyEvent {
    /// Phase.
    pub kind: KeyEventKind,
    /// Windows virtual-key code (what Chromium keys DOM `keyCode` off).
    pub windows_key_code: i32,
    /// Platform scan/key code (macOS `keyCode`).
    pub native_key_code: i32,
    /// The character produced, for [`KeyEventKind::Char`]. For a key-down
    /// and a key-up, the key's own character, and `None` for a modifier key.
    pub character: Option<char>,
    /// Modifier keys.
    pub modifiers: Modifiers,
    /// The editing commands the press carries, on a key-down that has any.
    pub editing: Option<EditingKey>,
}

/// The editing commands of one key press, which a page runs as the key's
/// default action unless its own handler takes the key.
///
/// A text field does not learn "word left" or "select all" from the key. The
/// platform's key bindings and its Edit menu turn the key into commands, and
/// a browser sends them along with it. A page fed the bare key has only the
/// few bindings Chromium keeps on every platform (the plain arrows,
/// Backspace). The devtools protocol is the one way in that takes commands,
/// and it wants the key's DOM names as well.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditingKey {
    /// The commands, in order: `AppKit` selector names without the colon
    /// (`moveWordLeft`, `deleteToBeginningOfLine`, `selectAll`).
    pub commands: Vec<String>,
    /// DOM `KeyboardEvent.code`, or empty for a key with no name here.
    pub code: &'static str,
    /// DOM `KeyboardEvent.key`.
    pub key: String,
}

/// An Edit menu command, done to whatever has the focus inside a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageEdit {
    /// Undo.
    Undo,
    /// Redo.
    Redo,
    /// Cut.
    Cut,
    /// Copy.
    Copy,
    /// Paste.
    Paste,
    /// Paste as plain text.
    PasteAndMatchStyle,
    /// Select all.
    SelectAll,
}

impl PageEdit {
    /// The command's name among a key's
    /// [`commands`](KeyEvent::commands): the `AppKit` selector an Edit menu
    /// sends, without its colon.
    pub const fn command(self) -> &'static str {
        match self {
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::Cut => "cut",
            Self::Copy => "copy",
            Self::Paste => "paste",
            Self::PasteAndMatchStyle => "pasteAndMatchStyle",
            Self::SelectAll => "selectAll",
        }
    }
}

/// Input-method events (CEF `ImeSetComposition` and friends). Ranges are in
/// UTF-16 code units, as CEF expects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeEvent {
    /// Update the in-progress composition (marked text).
    SetComposition {
        /// Marked text.
        text: String,
        /// Caret/selection within `text`.
        selection: Range<u32>,
        /// Existing document text the composition replaces, if any.
        replacement: Option<Range<u32>>,
    },
    /// Commit text, ending any composition.
    Commit {
        /// Committed text.
        text: String,
        /// Existing document text the commit replaces, if any.
        replacement: Option<Range<u32>>,
    },
    /// Commit the current composition as-is.
    FinishComposing {
        /// Keep the composition selected after committing.
        keep_selection: bool,
    },
    /// Abandon the current composition.
    Cancel,
}

/// Any input a page can receive.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// Pointer move/press/release/leave.
    Pointer(PointerEvent),
    /// Scroll.
    Wheel(WheelEvent),
    /// Keyboard.
    Key(KeyEvent),
    /// Input method.
    Ime(ImeEvent),
    /// An Edit menu command chosen with no key: from the menu bar.
    Edit(PageEdit),
}
