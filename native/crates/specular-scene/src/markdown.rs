//! Markdown as a flat list of blocks, ready to be set as rows of text.
//!
//! [`parse`] reads `CommonMark` with GFM tables, task lists and strikethrough.
//! Nesting is flattened: each [`Block`] says how many block quotes and how
//! many lists it sits inside, so laying a document out is one pass down the
//! list with no recursion. Inline markup becomes [`InlineSpan`]s over the
//! block's text, with the markup characters gone.

mod parse;
#[cfg(test)]
mod tests;

use std::ops::Range;

pub(crate) use parse::parse;

/// One block of a document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Block {
    /// What the block is.
    pub(crate) kind: BlockKind,
    /// How many block quotes it is inside.
    pub(crate) quote: u8,
    /// How many lists it is inside. A list item hangs its marker at this
    /// depth; any other block at a depth above zero continues an item.
    pub(crate) indent: u8,
    /// Whether it follows the block before it with no space between: an item
    /// of a tight list after the list's first.
    pub(crate) tight: bool,
}

/// What a [`Block`] is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BlockKind {
    /// A heading, levels 1 to 6.
    Heading {
        /// The level.
        level: u8,
        /// Its text.
        text: Inline,
    },
    /// A paragraph.
    Paragraph(Inline),
    /// The first paragraph of a list item, with the item's marker.
    Item {
        /// The marker hung beside it.
        marker: Marker,
        /// Its text.
        text: Inline,
    },
    /// A fenced or indented code block, without its last line break.
    Code(String),
    /// A thematic break.
    Rule,
    /// A table.
    Table(Table),
}

/// What is hung beside a list item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Marker {
    /// An unordered item.
    Bullet,
    /// An ordered item's number.
    Number(u64),
    /// A task, ticked or not.
    Task {
        /// Whether it is ticked.
        done: bool,
    },
}

/// A table: a head row and body rows of the same width.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Table {
    /// How each column is aligned.
    pub(crate) columns: Vec<ColumnAlign>,
    /// The head cells.
    pub(crate) head: Vec<Inline>,
    /// The body rows.
    pub(crate) rows: Vec<Vec<Inline>>,
}

/// How a table column is aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnAlign {
    /// To the left, which is also what an unmarked column gets.
    Left,
    /// Centred.
    Centre,
    /// To the right.
    Right,
}

/// A run of inline content: its text with the markup removed, and the
/// stretches that are styled.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Inline {
    /// The text. A hard break is a line feed; a soft break is a space.
    pub(crate) text: String,
    /// The styled stretches, in order and not overlapping. Text outside
    /// them is plain.
    pub(crate) spans: Vec<InlineSpan>,
}

/// A stretch of an [`Inline`] in one style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InlineSpan {
    /// The bytes of the text it covers.
    pub(crate) range: Range<usize>,
    /// How it is styled.
    pub(crate) style: InlineStyle,
}

/// The inline styles on a stretch of text. Any of them can combine.
#[expect(
    clippy::struct_excessive_bools,
    reason = "six independent flags, as markdown has them"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct InlineStyle {
    /// `**strong**`.
    pub(crate) strong: bool,
    /// `*emphasis*`.
    pub(crate) emphasis: bool,
    /// `~~strikethrough~~`.
    pub(crate) strike: bool,
    /// `` `code` ``.
    pub(crate) code: bool,
    /// `[text](url)`.
    pub(crate) link: bool,
    /// `![alt](url)`, standing in for the picture as `[image: alt]`.
    pub(crate) image: bool,
}
