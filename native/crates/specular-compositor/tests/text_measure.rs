//! `GlyphMeasure` with no GPU: lines, wrapping, caret stops and where a
//! click lands, on the system fonts. Glyph widths depend on the installed
//! fonts, so the tests compare a layout with itself and with the same text
//! set another way, never with fixed numbers.

use specular_compositor::GlyphMeasure;
use specular_core::text::{TextAlign, TextFont};
use specular_interact::{TextLayout, TextMeasure, TextSpec};

const LINE: f32 = 24.0;

fn spec(wrap_width: Option<f32>) -> TextSpec {
    TextSpec {
        font: TextFont::Sans,
        size: 16.0,
        line_height: LINE,
        wrap_width,
        align: TextAlign::Left,
    }
}

fn ranges(layout: &TextLayout) -> Vec<(usize, usize)> {
    (layout.lines.iter())
        .map(|line| (line.range.start, line.range.end))
        .collect()
}

/// The width of `text` on one line.
fn width_of(measure: &GlyphMeasure, text: &str) -> f32 {
    measure.layout(text, &spec(None)).width()
}

#[test]
fn words_wrap_at_the_width_and_the_lines_leave_no_byte_out() {
    let measure = GlyphMeasure::new();
    let text = "aaaa bbbb cccc dddd";
    // Room for two words and not for a third.
    let wrap = width_of(&measure, "aaaa bbbb") + 2.0;
    let layout = measure.layout(text, &spec(Some(wrap)));
    // The space a line broke at stays on that line.
    assert_eq!(ranges(&layout), [(0, 10), (10, 19)]);
    assert_eq!(layout.height(), LINE * 2.0);
    assert_eq!(layout.lines[1].stops[0].x, 0.0);
    let last = layout.lines[1].stops.last().map(|stop| stop.x);
    assert!(last.is_some_and(|x| x <= wrap), "{last:?} within {wrap}");
}
