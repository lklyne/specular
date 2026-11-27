//! [`Tool`]: what the next click or drag on the canvas does.

use crate::Cursor;
use crate::mods::ModToolId;

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
    /// Read the DOM node under the pointer, and pick one with a click.
    Inspect,
    /// A tool a mod adds (ADR 0057): its stroke is the `tool` state key,
    /// and the mod previews and commits it.
    Mod(ModToolId),
}

impl Tool {
    /// Every tool, in toolbar order.
    pub(crate) const ALL: [Self; 9] = [
        Self::Select,
        Self::AddPage,
        Self::AddText,
        Self::AddSticky,
        Self::AddDocument,
        Self::AddShape,
        Self::Draw,
        Self::Comment,
        Self::Inspect,
    ];

    /// The word the tool is named by: its toolbar control is `tool.<name>`,
    /// and a script and the `tool.set` command take it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Select => "select",
            Self::AddPage => "page",
            Self::AddText => "text",
            Self::AddSticky => "sticky",
            Self::AddDocument => "document",
            Self::AddShape => "shape",
            Self::Draw => "draw",
            Self::Comment => "comment",
            Self::Inspect => "inspect",
            Self::Mod(_) => "mod",
        }
    }

    /// The tool [`name`](Self::name) gives `name`.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tool| tool.name() == name)
    }

    /// The cursor shown over the canvas while the tool is active.
    pub(crate) const fn cursor(self) -> Cursor {
        match self {
            Self::Select => Cursor::Default,
            Self::AddPage
            | Self::AddText
            | Self::AddSticky
            | Self::AddDocument
            | Self::AddShape
            | Self::Draw
            | Self::Comment
            | Self::Inspect
            | Self::Mod(_) => Cursor::Crosshair,
        }
    }
}
