//! [`Tool`]: what the next click or drag on the canvas does.

use crate::Cursor;

/// The active tool. Exactly one is active at a time. Variants such as the
/// shape kind or the brush live in tool defaults, not here (ADR 0009).
///
/// Match on this without a wildcard arm, so a new tool makes the compiler
/// list every place that must handle it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Tool {
    /// Select, move and resize. Escape returns here.
    #[default]
    Select,
    /// Place a page.
    AddPage,
    /// Place plain text.
    AddText,
    /// Place a sticky note.
    AddSticky,
    /// Place a markdown document.
    AddDocument,
    /// Drag out a shape.
    AddShape,
    /// Draw freehand strokes.
    Draw,
    /// Comment on a point, an element or a region.
    Comment,
}

impl Tool {
    /// Every tool, in toolbar order.
    pub const ALL: [Self; 8] = [
        Self::Select,
        Self::AddPage,
        Self::AddText,
        Self::AddSticky,
        Self::AddDocument,
        Self::AddShape,
        Self::Draw,
        Self::Comment,
    ];

    /// The cursor shown over the canvas while the tool is active.
    pub const fn cursor(self) -> Cursor {
        match self {
            Self::Select => Cursor::Default,
            Self::AddPage
            | Self::AddText
            | Self::AddSticky
            | Self::AddDocument
            | Self::AddShape
            | Self::Draw
            | Self::Comment => Cursor::Crosshair,
        }
    }
}
