//! Where the rows of a [`ColumnDraw`] land once their text is measured.
//! Pure, so stacking is tested without a font.

use specular_scene::{ColumnDraw, Rect, Row, RowRule, RuleHeight, Size, TextRun};

use super::text_layout::text_rect;

/// One row's place, measured down from the column's origin before scrolling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RowBox {
    /// Top of the row, under its gap.
    pub(crate) top: f32,
    pub(crate) height: f32,
    gap: f32,
}

/// A column with every row placed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ColumnLayout {
    pub(crate) rows: Vec<RowBox>,
    /// Height of all the rows and gaps.
    height: f32,
    /// How far up the rows are moved: the column's scroll, stopped at its end.
    shift: f32,
}

/// Stacks `column`'s rows. `measure` gives the size of a cell's shaped lines.
pub(crate) fn layout(
    column: &ColumnDraw,
    mut measure: impl FnMut(&TextRun) -> Size,
) -> ColumnLayout {
    let mut bottom = 0.0;
    let rows = (column.rows.iter())
        .map(|row| {
            let text = (row.cells.iter())
                .map(|cell| text_rect(cell, measure(cell)).bottom())
                .fold(0.0, f32::max);
            let row = RowBox {
                top: bottom + row.gap,
                height: (text + row.bottom_padding).max(row.min_height),
                gap: row.gap,
            };
            bottom = row.top + row.height;
            row
        })
        .collect();
    let end = (bottom - column.height).max(0.0);
    ColumnLayout {
        rows,
        height: bottom,
        shift: column.scroll.clamp(0.0, end),
    }
}

impl ColumnLayout {
    /// Height of all the rows and gaps.
    pub(crate) fn height(&self) -> f32 {
        self.height
    }

    /// Everything the column draws, in the item's space.
    pub(crate) fn bounds(&self, column: &ColumnDraw) -> Rect {
        Rect::new(
            column.origin.x,
            column.origin.y - self.shift,
            column.width,
            self.height,
        )
    }

    /// Where `row` is, in the item's space.
    pub(crate) fn row_rect(&self, column: &ColumnDraw, row: &RowBox) -> Rect {
        Rect::new(
            column.origin.x,
            column.origin.y + row.top - self.shift,
            column.width,
            row.height,
        )
    }

    /// The rows with their places, top to bottom.
    pub(crate) fn rows<'a>(
        &'a self,
        column: &'a ColumnDraw,
    ) -> impl Iterator<Item = (&'a Row, &'a RowBox)> {
        column.rows.iter().zip(&self.rows)
    }
}

/// Where `rule` is drawn, measured from the top-left of a `row`'s rect.
pub(crate) fn rule_rect(rule: &RowRule, row: &RowBox) -> Rect {
    let (y, height) = match rule.height {
        RuleHeight::Row => (0.0, row.height),
        RuleHeight::RowAndGap => (-row.gap, row.height + row.gap),
        RuleHeight::Middle(height) => ((row.height - height) * 0.5, height),
        RuleHeight::Bottom(height) => (row.height - height, height),
    };
    Rect::new(rule.x, y, rule.width, height)
}
