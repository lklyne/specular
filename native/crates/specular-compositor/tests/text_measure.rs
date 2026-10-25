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

fn offsets(layout: &TextLayout, line: usize) -> Vec<usize> {
    (layout.lines[line].stops.iter())
        .map(|stop| stop.offset)
        .collect()
}

/// Whether the stops of `line` step strictly rightwards.
fn runs_rightwards(layout: &TextLayout, line: usize) -> bool {
    let stops = &layout.lines[line].stops;
    stops.windows(2).all(|pair| pair[0].x < pair[1].x)
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

#[test]
fn an_emoji_sequence_is_one_stop_however_many_scalars_it_has() {
    let measure = GlyphMeasure::new();
    // A thumbs-up with a skin tone, and a family joined by zero-width joiners.
    let (thumb, family) = (
        "\u{1F44D}\u{1F3FD}",
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}",
    );
    let text = format!("a{thumb}b{family}");
    let layout = measure.layout(&text, &spec(None));
    let (after_thumb, after_b) = (1 + thumb.len(), 2 + thumb.len());
    assert_eq!(
        offsets(&layout, 0),
        [0, 1, after_thumb, after_b, after_b + family.len()]
    );
    assert!(runs_rightwards(&layout, 0), "{:?}", layout.lines[0].stops);
}
