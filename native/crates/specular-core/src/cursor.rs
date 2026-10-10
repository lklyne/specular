//! The pointer cursor: what the canvas asks the shell to show, and what a
//! page asks for over its own content.

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
    /// The hand over a link or a button.
    Pointer,
    /// A top-left or bottom-right resize handle.
    ResizeNwse,
    /// A top-right or bottom-left resize handle.
    ResizeNesw,
    /// A left or right edge that can be dragged.
    ResizeEw,
    /// A top or bottom edge that can be dragged.
    ResizeNs,
    /// A divider between columns.
    ResizeColumn,
    /// A divider between rows.
    ResizeRow,
    /// A text caret for vertical text.
    VerticalText,
    /// What is under the pointer cannot be acted on.
    NotAllowed,
    /// A drop here makes a link.
    Alias,
    /// A drop here makes a copy.
    Copy,
    /// A menu opens here.
    ContextMenu,
}
