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

#[cfg(test)]
mod tests {
    use specular_scene::{Color, Point};

    use super::*;

    /// Every cell measures 50 wide and one 20-unit line per 10 characters.
    fn measure(run: &TextRun) -> Size {
        Size::new(50.0, run.text.len().div_ceil(10) as f32 * 20.0)
    }

    fn cell(text: &str, y: f32) -> TextRun {
        TextRun::new(text, Point::new(0.0, y), 14.0, Color::BLACK)
    }

    fn column(scroll: f32, rows: Vec<Row>) -> ColumnDraw {
        ColumnDraw {
            origin: Point::new(100.0, 200.0),
            width: 300.0,
            height: 50.0,
            scroll,
            rows,
            owner: None,
        }
    }

    fn rows() -> Vec<Row> {
        vec![
            Row {
                cells: vec![cell("one line", 0.0)],
                ..Row::default()
            },
            Row {
                gap: 8.0,
                cells: vec![cell("a", 0.0), cell("three lines of text here", 4.0)],
                ..Row::default()
            },
            Row {
                gap: 8.0,
                min_height: 10.0,
                bottom_padding: 6.0,
                ..Row::default()
            },
            Row {
                cells: vec![cell("x", 0.0)],
                bottom_padding: 5.0,
                ..Row::default()
            },
        ]
    }

    fn tops_and_heights(layout: &ColumnLayout) -> Vec<(f32, f32)> {
        (layout.rows.iter())
            .map(|row| (row.top, row.height))
            .collect()
    }

    #[test]
    fn rows_stack_under_their_gaps_and_are_as_tall_as_their_cells_padding_and_minimum() {
        let column = column(0.0, rows());
        let layout = layout(&column, measure);
        assert_eq!(
            tops_and_heights(&layout),
            [(0.0, 20.0), (28.0, 64.0), (100.0, 10.0), (110.0, 25.0)]
        );
        assert_eq!(
            layout.bounds(&column),
            Rect::new(100.0, 200.0, 300.0, 135.0)
        );
    }

    #[test]
    fn rules_sit_against_their_row() {
        let row = RowBox {
            top: 28.0,
            height: 64.0,
            gap: 8.0,
        };
        let rect = |height| {
            let rule = RowRule {
                x: 4.0,
                width: 2.0,
                height,
                color: Color::BLACK,
            };
            let rect = rule_rect(&rule, &row);
            (rect.y, rect.height)
        };
        assert_eq!(rect(RuleHeight::Row), (0.0, 64.0));
        assert_eq!(rect(RuleHeight::RowAndGap), (-8.0, 72.0));
        assert_eq!(rect(RuleHeight::Middle(2.0)), (31.0, 2.0));
        assert_eq!(rect(RuleHeight::Bottom(1.0)), (63.0, 1.0));
    }
}
