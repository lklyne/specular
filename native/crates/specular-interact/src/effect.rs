//! [`Effect`]: the I/O [`update`](crate::update) asks the shell to do.

use glam::Vec2;
use specular_core::{CssSize, InputEvent};
use specular_doc::EntityId;

use crate::{AssetBytes, ImageKey, ToolDefaults};

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
    /// Put text on the system clipboard: the selected text of an edit, or
    /// the selected entities as [`copy`](crate::Action::Copy) writes them.
    WriteClipboard(String),
    /// Read the system clipboard and answer with
    /// [`Event::Clipboard`](crate::Event::Clipboard).
    ReadClipboard,
    /// Write `bytes` to a new file in the space folder. It comes before the
    /// effect that loads the file.
    WriteAsset {
        /// Where, relative to the space folder.
        file: String,
        /// The file's contents.
        bytes: AssetBytes,
    },
    /// Copy a file from outside the space folder into it. It comes before
    /// the effect that loads the copy.
    CopyAsset {
        /// The absolute path of the file to copy.
        from: String,
        /// Where the copy goes, relative to the space folder.
        file: String,
    },
    /// Decode an image file and upload it under `image`, then answer with
    /// [`Event::Image`](crate::Event::Image).
    LoadImage {
        /// The key the answer and the renderer use.
        image: ImageKey,
        /// The path as the document writes it: relative to the space folder,
        /// absolute, or a URL.
        file: String,
    },
    /// Forget an image: nothing shows it any more.
    DropImage(ImageKey),
    /// Read a markdown file and answer with
    /// [`Event::Note`](crate::Event::Note), then again whenever the file
    /// changes on disk.
    LoadNote {
        /// The path as the document writes it, which the answer repeats.
        file: String,
    },
    /// Stop watching a markdown file: nothing shows it any more.
    DropNote {
        /// The path as the document writes it.
        file: String,
    },
    /// Write the tool defaults to the preferences file, under `toolDefaults`,
    /// as [`ToolDefaults::to_json`] gives them.
    SaveToolDefaults(Box<ToolDefaults>),
}

/// A pointer cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Cursor {
    /// The arrow.
    #[default]
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
