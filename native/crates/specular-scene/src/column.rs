//! Columns: rows of text stacked top to bottom, each as tall as its text
//! turns out to be. Only the renderer measures text, so `view` says what the
//! rows hold and the renderer finds where each one starts.

use crate::{Color, Point, TextRun};

/// Rows of text stacked top to bottom from `origin`.
///
/// A document is one of these: each paragraph, heading and list item is a
/// row. Clip the item to cut the column off at a card's edge.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnDraw {
    /// Top-left of the first row, before scrolling.
    pub origin: Point,
    /// Width of the column. Rules are placed inside it; a cell wraps at its
    /// own `wrap_width`.
    pub width: f32,
    /// Height the rows show through. It only bounds `scroll`.
    pub height: f32,
    /// How far the rows are moved up. The renderer stops at the point where
    /// the last row's bottom reaches `height`, so a value past the end shows
    /// the end.
    pub scroll: f32,
    /// The rows, top to bottom.
    pub rows: Vec<Row>,
}

/// One row of a [`ColumnDraw`]. It is as tall as its tallest cell.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Row {
    /// Space above the row.
    pub gap: f32,
    /// The least height of the row, for a row with no text.
    pub min_height: f32,
    /// Room inside the row under its tallest cell.
    pub bottom_padding: f32,
    /// The text in the row. Each run's `origin` is measured from the row's
    /// top-left corner. Runs are top-aligned; `box_height` and
    /// `vertical_align` are not used.
    pub cells: Vec<TextRun>,
    /// Straight lines drawn with the row: a quote's bar, a divider, a table
    /// border.
    pub rules: Vec<RowRule>,
}

/// A thin filled rect placed against a [`Row`]. The renderer draws it with
/// the text, so it is meant for lines a few units thick, not for fills.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowRule {
    /// Left edge, measured from the column's left.
    pub x: f32,
    /// Width.
    pub width: f32,
    /// Where it sits in the row and how tall it is.
    pub height: RuleHeight,
    /// Fill colour.
    pub color: Color,
}

/// The vertical extent of a [`RowRule`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuleHeight {
    /// The whole row.
    Row,
    /// The whole row and the gap above it, so the rules of neighbouring rows
    /// join up.
    RowAndGap,
    /// This tall, centred in the row.
    Middle(f32),
    /// This tall, along the bottom of the row.
    Bottom(f32),
}
