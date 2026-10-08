//! What the editor reads off a [`TextLayout`]: which line an offset is on,
//! where a point lands, and the boxes of the caret and of a range.
//!
//! An offset where a wrapped line ends is also where the next line starts.
//! It belongs to the next line, so the end of a wrapped line, for the End
//! key and for a click past its last glyph, is the stop before that one:
//! usually just before the space the line broke at.

use std::ops::Range;

use super::{CaretStop, LayoutLine, TextLayout};

/// A box on one line, in the layout's own space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineBox {
    /// The left edge.
    pub left: f32,
    /// The right edge.
    pub right: f32,
    /// The top of the line.
    pub top: f32,
    /// The height of the line.
    pub height: f32,
}

impl TextLayout {
    /// The height of all the lines together.
    pub fn height(&self) -> f32 {
        (self.lines.last()).map_or(0.0, |line| line.top + line.height)
    }

    /// How far the lines reach to the right of the layout's left edge.
    pub fn width(&self) -> f32 {
        let stops = self.lines.iter().flat_map(|line| &line.stops);
        stops.map(|stop| stop.x).fold(0.0, f32::max)
    }

    /// The index of the line `offset` is on.
    pub fn line_of(&self, offset: usize) -> usize {
        let after = self
            .lines
            .partition_point(|line| line.range.start <= offset);
        after.saturating_sub(1)
    }

    fn line(&self, index: usize) -> Option<&LayoutLine> {
        self.lines.get(index)
    }

    /// Whether line `index` ends because it was wrapped, so its end offset
    /// is the next line's first.
    fn is_wrapped(&self, index: usize) -> bool {
        match (self.line(index), self.line(index + 1)) {
            (Some(line), Some(next)) => next.range.start == line.range.end,
            _ => false,
        }
    }

    /// The stops of line `index` the caret can rest on while staying on it.
    fn resting_stops(&self, index: usize) -> &[CaretStop] {
        let Some(line) = self.line(index) else {
            return &[];
        };
        match line.stops.split_last() {
            Some((_, rest)) if self.is_wrapped(index) && !rest.is_empty() => rest,
            _ => &line.stops,
        }
    }

    /// Where the caret is drawn for `offset`.
    pub fn x_of(&self, offset: usize) -> f32 {
        let Some(line) = self.line(self.line_of(offset)) else {
            return 0.0;
        };
        let at = line.stops.partition_point(|stop| stop.offset <= offset);
        let stop = line.stops.get(at.saturating_sub(1));
        stop.map_or(0.0, |stop| stop.x)
    }

    /// The offset on line `index` whose caret is nearest `x`.
    pub fn offset_at(&self, index: usize, x: f32) -> usize {
        let nearest = (self.resting_stops(index).iter())
            .min_by(|a, b| (a.x - x).abs().total_cmp(&(b.x - x).abs()));
        match (nearest, self.line(index)) {
            (Some(stop), _) => stop.offset,
            (None, Some(line)) => line.range.start,
            (None, None) => 0,
        }
    }

    /// The index of the line at height `y`, held to the first and last.
    pub fn line_at(&self, y: f32) -> usize {
        let below = self.lines.partition_point(|line| line.top <= y);
        below.saturating_sub(1)
    }

    /// The offset nearest the point `(x, y)`.
    pub fn offset_at_point(&self, x: f32, y: f32) -> usize {
        self.offset_at(self.line_at(y), x)
    }

    /// The first offset of the line `offset` is on.
    pub fn line_start(&self, offset: usize) -> usize {
        let line = self.line(self.line_of(offset));
        line.map_or(0, |line| line.range.start)
    }

    /// The last offset the caret can rest at on the line `offset` is on.
    pub fn line_end(&self, offset: usize) -> usize {
        let index = self.line_of(offset);
        match (self.resting_stops(index).last(), self.line(index)) {
            (Some(stop), _) => stop.offset,
            (None, Some(line)) => line.range.end,
            (None, None) => offset,
        }
    }

    /// The caret's box at `offset`: no width, one line tall.
    pub fn caret_box(&self, offset: usize) -> Option<LineBox> {
        let line = self.line(self.line_of(offset))?;
        let x = self.x_of(offset);
        Some(LineBox {
            left: x,
            right: x,
            top: line.top,
            height: line.height,
        })
    }

    /// One box per line `range` touches. A line whose break is inside the
    /// range reaches a little past its last glyph, to show the break taken.
    pub fn range_boxes(&self, range: &Range<usize>) -> Vec<LineBox> {
        let mut boxes = Vec::new();
        if range.is_empty() {
            return boxes;
        }
        for (index, line) in self.lines.iter().enumerate() {
            let start = range.start.max(line.range.start);
            let end = range.end.min(line.range.end);
            let takes_break = range.end > line.range.end && !self.is_wrapped(index);
            if start > end || line.range.start >= range.end || (start == end && !takes_break) {
                continue;
            }
            let inside = (line.stops.iter())
                .filter(|stop| (start..=end).contains(&stop.offset))
                .map(|stop| stop.x);
            let (left, right) = inside.fold((f32::MAX, f32::MIN), |(left, right), x| {
                (left.min(x), right.max(x))
            });
            if left > right {
                continue;
            }
            let tail = if takes_break { line.height * 0.3 } else { 0.0 };
            boxes.push(LineBox {
                left,
                right: right + tail,
                top: line.top,
                height: line.height,
            });
        }
        boxes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "ab cd" wrapped after the space, then a hard break, then "e": ten
    /// units a character, twenty a line.
    fn layout() -> TextLayout {
        let line = |range: Range<usize>, row: usize| LayoutLine {
            stops: (range.start..=range.end)
                .map(|offset| CaretStop {
                    offset,
                    x: (offset - range.start) as f32 * 10.0,
                })
                .collect(),
            range,
            top: row as f32 * 20.0,
            height: 20.0,
        };
        TextLayout {
            lines: vec![line(0..3, 0), line(3..5, 1), line(6..7, 2)],
        }
    }

    #[test]
    fn an_offset_at_a_wrap_belongs_to_the_next_line() {
        let layout = layout();
        assert_eq!(
            [0, 2, 3, 5, 6, 7].map(|offset| layout.line_of(offset)),
            [0, 0, 1, 1, 2, 2]
        );
        assert_eq!(layout.x_of(3), 0.0);
        assert_eq!(layout.line_end(1), 2);
        assert_eq!(layout.line_end(3), 5, "a hard break ends at its last byte");
        assert_eq!(layout.offset_at(0, 500.0), 2);
        assert_eq!(layout.offset_at(1, 500.0), 5);
    }

    #[test]
    fn a_point_lands_on_the_nearest_stop_of_the_line_under_it() {
        let layout = layout();
        let rows = [
            ("inside the first line", (14.0, 5.0), 1),
            ("inside the second line", (16.0, 25.0), 5),
            ("on the second line's top edge", (16.0, 20.0), 5),
            ("above the text", (0.0, -50.0), 0),
            ("below the text", (90.0, 900.0), 7),
        ];
        for (name, (x, y), offset) in rows {
            assert_eq!(layout.offset_at_point(x, y), offset, "{name}");
        }
    }

    #[test]
    fn a_range_is_one_box_a_line_and_shows_a_hard_break_it_takes() {
        let layout = layout();
        let boxes = layout.range_boxes(&(1..7));
        assert_eq!(
            boxes
                .iter()
                .map(|line| (line.left, line.right, line.top))
                .collect::<Vec<_>>(),
            [(10.0, 30.0, 0.0), (0.0, 26.0, 20.0), (0.0, 10.0, 40.0)]
        );
        assert_eq!(layout.range_boxes(&(2..2)), []);
        assert_eq!((layout.height(), layout.width()), (60.0, 30.0));
    }
}
