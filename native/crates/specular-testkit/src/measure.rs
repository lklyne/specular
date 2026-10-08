//! [`FixedAdvance`]: a text measure with no fonts in it, so a test can say
//! where a caret is in round numbers.

use specular_doc::TextAlign;
use specular_interact::{CaretStop, LayoutLine, TextLayout, TextMeasure, TextSpec};
use unicode_segmentation::UnicodeSegmentation;

/// Lays text out with every grapheme the same width and every line the same
/// height, whatever the font and size. The default is 10 wide and 20 tall,
/// which every [`TestApp`](crate::TestApp) starts with.
///
/// Wrapping is greedy. A line breaks after the last space that fits, the
/// spaces at a break stay on the line they end, and a word longer than the
/// line breaks wherever it has to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedAdvance {
    /// The width of every grapheme, in canvas units.
    pub advance: f32,
    /// The height of every line, in canvas units.
    pub line_height: f32,
}

impl Default for FixedAdvance {
    fn default() -> Self {
        Self {
            advance: 10.0,
            line_height: 20.0,
        }
    }
}

impl FixedAdvance {
    /// Where the visual lines of one paragraph end, as counts of its
    /// graphemes. `fits` is how many a line holds.
    fn breaks(graphemes: &[&str], fits: usize) -> Vec<usize> {
        let mut ends = Vec::new();
        let (mut start, mut after_space) = (0, None);
        for (index, grapheme) in graphemes.iter().enumerate() {
            let is_space = grapheme.chars().all(char::is_whitespace);
            if !is_space && index - start >= fits {
                // Break after the last space on the line, or right here in
                // the middle of a word that has no space to break at.
                start = after_space.unwrap_or(index);
                ends.push(start);
                after_space = None;
            }
            if is_space {
                after_space = Some(index + 1);
            }
        }
        ends.push(graphemes.len());
        ends
    }
}

impl TextMeasure for FixedAdvance {
    fn layout(&self, text: &str, spec: &TextSpec) -> TextLayout {
        let fits = spec.wrap_width.map_or(usize::MAX, |width| {
            ((width / self.advance).floor() as usize).max(1)
        });
        let mut lines = Vec::new();
        let mut paragraph_start = 0;
        for paragraph in text.split('\n') {
            let graphemes: Vec<&str> = paragraph.graphemes(true).collect();
            let mut offsets = vec![paragraph_start];
            for grapheme in &graphemes {
                offsets.push(offsets[offsets.len() - 1] + grapheme.len());
            }
            let mut from = 0;
            for to in Self::breaks(&graphemes, fits) {
                // Spaces left hanging at a break take no room when the line
                // is aligned.
                let inked = graphemes[from..to]
                    .iter()
                    .rposition(|grapheme| !grapheme.chars().all(char::is_whitespace))
                    .map_or(0, |last| last + 1);
                let width = inked as f32 * self.advance;
                let left = match (spec.align, spec.wrap_width) {
                    (TextAlign::Left, _) => 0.0,
                    (TextAlign::Center, extent) => (extent.unwrap_or(0.0) - width) / 2.0,
                    (TextAlign::Right, extent) => extent.unwrap_or(0.0) - width,
                };
                lines.push(LayoutLine {
                    range: offsets[from]..offsets[to],
                    top: lines.len() as f32 * self.line_height,
                    height: self.line_height,
                    stops: (from..=to)
                        .map(|index| CaretStop {
                            offset: offsets[index],
                            x: left + (index - from) as f32 * self.advance,
                        })
                        .collect(),
                });
                from = to;
            }
            paragraph_start += paragraph.len() + 1;
        }
        TextLayout { lines }
    }
}

#[cfg(test)]
mod tests {
    use specular_doc::TextFont;

    use super::*;

    fn lines(text: &str, wrap_width: Option<f32>) -> Vec<&str> {
        let spec = TextSpec {
            font: TextFont::Sans,
            size: 14.0,
            line_height: 21.0,
            wrap_width,
            align: TextAlign::Left,
        };
        let layout = FixedAdvance::default().layout(text, &spec);
        (layout.lines.iter())
            .map(|line| &text[line.range.clone()])
            .collect()
    }

    #[test]
    fn lines_break_after_the_last_space_that_fits() {
        assert_eq!(lines("one two three", Some(80.0)), ["one two ", "three"]);
        assert_eq!(
            lines("one two three", Some(50.0)),
            ["one ", "two ", "three"]
        );
        assert_eq!(lines("one two", None), ["one two"]);

        {
            assert_eq!(
                lines("abcdefgh ij", Some(30.0)),
                ["abc", "def", "gh ", "ij"]
            );
        }

        {
            assert_eq!(lines("a\n\nb\n", Some(80.0)), ["a", "", "b", ""]);
            assert_eq!(lines("", None), [""]);
        }
    }
}
