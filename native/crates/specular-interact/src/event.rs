//! [`Event`]: everything that can happen to an [`App`](crate::App).

use glam::Vec2;
use specular_core::{Camera, ImeEvent, Modifiers, PixelRect, PointerEventKind};
use specular_doc::{Document, EntityId, ItemId};

use crate::{ImageKey, ImageNotice, NoteNotice, Tool, ToolDefaultPatch, ToolDefaults};

/// One input to [`update`](crate::update). Window input arrives in logical
/// screen pixels, origin at the canvas viewport's top-left.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// The pointer moved, pressed, released or left the window.
    Pointer(PointerInput),
    /// A wheel or two-finger scroll.
    Wheel(WheelInput),
    /// A trackpad pinch. `delta` is the change in magnification; positive
    /// zooms in.
    Pinch {
        /// The change in magnification.
        delta: f32,
    },
    /// A key went down or up.
    Key(KeyInput),
    /// The OS input method composed or committed text.
    Ime(ImeEvent),
    /// A hosted page reported something.
    Page {
        /// The page entity.
        page: EntityId,
        /// What it reported.
        notice: PageNotice,
    },
    /// The shell finished with an image an
    /// [`Effect::LoadImage`](crate::Effect::LoadImage) asked for.
    Image {
        /// The image.
        image: ImageKey,
        /// How it went.
        notice: ImageNotice,
    },
    /// The shell read a markdown file an
    /// [`Effect::LoadNote`](crate::Effect::LoadNote) asked for, or saw it
    /// change on disk.
    Note {
        /// The path as the document writes it.
        file: String,
        /// What the file holds now.
        notice: NoteNotice,
    },
    /// The wall clock, sent once per loop turn. Milliseconds since the Unix
    /// epoch.
    Tick {
        /// The time.
        unix_ms: u64,
    },
    /// The canvas viewport changed size, in logical pixels.
    ViewportResized(Vec2),
    /// A document was loaded: at startup, on switching canvas, or when the
    /// file changed on disk. Replaces the current one and clears the history.
    DocumentOpened(Box<Document>),
    /// The tool defaults were read from the preferences file. Replaces the
    /// current ones and asks for no save.
    ToolDefaultsLoaded(Box<ToolDefaults>),
    /// A command from a key binding, a menu, a panel or the HTTP API.
    Action(Action),
}

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
    /// Modifier keys held.
    pub modifiers: Modifiers,
    /// The Windows virtual-key code, which Chromium derives DOM `keyCode`
    /// from on every platform. Only used when the key goes to a page.
    pub windows_key_code: i32,
    /// The platform scan code, which Chromium derives DOM `code` from. Only
    /// used when the key goes to a page.
    pub native_key_code: i32,
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

/// Something a hosted page reported that the interaction layer acts on.
/// Painted frames are not here: they go straight to the renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageNotice {
    /// The main frame finished loading.
    Loaded {
        /// The HTTP status, 0 for a non-HTTP load.
        http_status: i32,
    },
    /// The page's process went away.
    Crashed {
        /// The backend's reason, such as `crashed` or `oom`.
        reason: String,
    },
    /// The IME composition moved. Bounds are in the page's CSS pixels;
    /// `None` when there is no composition.
    ImeCompositionBounds(Option<PixelRect>),
}

/// A command with no pointer position: what a key binding, a menu item, a
/// toolbar button or an API route asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Escape: abandon the gesture in flight, return to the select tool and
    /// leave the entered page. With none of those to back out of, clear the
    /// selection.
    Cancel,
    /// Switch tool.
    SetTool(Tool),
    /// Change one tool default and save the defaults.
    SetToolDefault(ToolDefaultPatch),
    /// Switch to the tool a default belongs to and change that default: what
    /// a variant key such as Shift+R or Shift+M does.
    SetToolVariant(ToolDefaultPatch),
    /// Undo the latest document step.
    Undo,
    /// Redo the latest undone step.
    Redo,
    /// Replace the selection. Ids that name nothing are dropped.
    Select(Vec<ItemId>),
    /// Move the camera.
    SetCamera(Camera),
    /// Remove the selection, with what is inside its groups, what is hooked
    /// to its pages and the edges that would lose an end.
    Delete,
    /// Copy the selection into free space beside it and select the copies.
    Duplicate,
    /// Move the selection by exactly this many canvas units.
    Nudge {
        /// Along x. Positive is right.
        dx: f64,
        /// Along y. Positive is down.
        dy: f64,
    },
}
