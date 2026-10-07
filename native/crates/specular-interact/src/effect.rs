//! [`Effect`]: the I/O [`update`](crate::update) asks the shell to do.

use glam::Vec2;
use specular_core::{CssSize, InputEvent};
use specular_doc::EntityId;

/// One thing for the shell to do after an [`update`](crate::update). Effects
/// run in the order they are returned.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    /// Start hosting a page entity.
    CreatePage {
        /// The page entity.
        page: EntityId,
        /// The URL to load.
        url: String,
        /// The layout viewport in CSS pixels.
        viewport: CssSize,
    },
    /// Stop hosting a page entity.
    ClosePage(EntityId),
    /// Re-lay-out a hosted page at a new viewport.
    SetPageViewport {
        /// The page entity.
        page: EntityId,
        /// The new layout viewport in CSS pixels.
        viewport: CssSize,
    },
    /// Give a page keyboard focus, or take it from every page.
    FocusPage(Option<EntityId>),
    /// Send one input event into a page, in its CSS pixels.
    ForwardInput {
        /// The page entity.
        page: EntityId,
        /// The event.
        event: InputEvent,
    },
    /// Turn the OS input method on or off for the window.
    SetImeAllowed(bool),
    /// Put the OS candidate window next to this rect, in logical screen
    /// pixels.
    SetImeCursorArea {
        /// Top-left corner.
        origin: Vec2,
        /// Width and height.
        size: Vec2,
    },
    /// Show this cursor over the canvas.
    SetCursor(Cursor),
    /// Write the document to its file. Nothing returns this until autosave
    /// (task S9) does.
    Save,
    /// Put text on the system clipboard. Nothing returns this until copy and
    /// cut (task S6) do.
    WriteClipboard(String),
}

/// A pointer cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cursor {
    /// The arrow.
    Default,
    /// A creation tool is armed.
    Crosshair,
    /// The canvas can be dragged.
    Grab,
    /// The canvas is being dragged.
    Grabbing,
    /// An item can be moved.
    Move,
    /// A text caret.
    Text,
    /// A top-left or bottom-right resize handle.
    ResizeNwse,
    /// A top-right or bottom-left resize handle.
    ResizeNesw,
}
