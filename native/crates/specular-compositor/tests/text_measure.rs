//! `GlyphMeasure` with no GPU: lines, wrapping, caret stops and where a
//! click lands, on the system fonts. Glyph widths depend on the installed
//! fonts, so the tests compare a layout with itself and with the same text
//! set another way, never with fixed numbers.

use std::sync::Arc;

use glam::Vec2;
use specular_compositor::GlyphMeasure;
use specular_core::text::{TextAlign, TextFont};
use specular_doc::{Entity, Kind, Rect, Text, TextStyle, WidthMode};
use specular_interact::{TextLayout, TextMeasure, TextSpec};
use specular_testkit::TestApp;

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
fn empty_text_is_one_line_with_one_stop() {
    let layout = GlyphMeasure::new().layout("", &spec(Some(200.0)));
    assert_eq!(ranges(&layout), [(0, 0)]);
    let stops = &layout.lines[0].stops;
    assert_eq!((stops.len(), stops[0].offset, stops[0].x), (1, 0, 0.0));
    assert_eq!(layout.height(), LINE);
}

#[test]
fn a_line_break_starts_a_line_and_a_trailing_one_leaves_an_empty_line() {
    let layout = GlyphMeasure::new().layout("ab\n\ncd\n", &spec(None));
    assert_eq!(ranges(&layout), [(0, 2), (3, 3), (4, 6), (7, 7)]);
    assert_eq!(offsets(&layout, 2), [4, 5, 6]);
    let tops: Vec<f32> = layout.lines.iter().map(|line| line.top).collect();
    assert_eq!(tops, [0.0, LINE, LINE * 2.0, LINE * 3.0]);
    assert_eq!(
        layout.lines[2].stops[0].x, 0.0,
        "each line starts at the left"
    );
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
fn a_word_too_long_for_the_width_breaks_inside_itself() {
    let measure = GlyphMeasure::new();
    let text = "m".repeat(30);
    let wrap = width_of(&measure, "mmmmmmmmmm") + 2.0;
    let layout = measure.layout(&text, &spec(Some(wrap)));
    assert_eq!(ranges(&layout), [(0, 10), (10, 20), (20, 30)]);
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

#[test]
fn cjk_text_has_a_stop_a_character_and_wraps_between_any_two() {
    let measure = GlyphMeasure::new();
    let text = "日本語日本語";
    let unwrapped = measure.layout(text, &spec(None));
    assert_eq!(offsets(&unwrapped, 0), [0, 3, 6, 9, 12, 15, 18]);
    assert!(runs_rightwards(&unwrapped, 0));
    // Room for three and a half characters.
    let wrap = unwrapped.width() * 3.5 / 6.0;
    let wrapped = measure.layout(text, &spec(Some(wrap)));
    assert_eq!(ranges(&wrapped), [(0, 9), (9, 18)]);
    assert_eq!(offsets(&wrapped, 1), [9, 12, 15, 18]);
}

#[test]
fn alignment_moves_the_stops_inside_a_width_and_around_an_anchor() {
    let measure = GlyphMeasure::new();
    let width = width_of(&measure, "ab");
    let ends = |wrap_width: Option<f32>, align: TextAlign, text: &str| {
        let aligned = TextSpec {
            align,
            ..spec(wrap_width)
        };
        let layout = measure.layout(text, &aligned);
        let stops = &layout.lines[0].stops;
        (stops[0].x, stops[stops.len() - 1].x)
    };
    let near = |(left, right): (f32, f32), (want_left, want_right): (f32, f32)| {
        (left - want_left).abs() < 0.01 && (right - want_right).abs() < 0.01
    };
    let centred = ends(Some(200.0), TextAlign::Center, "ab");
    assert!(
        near(centred, (100.0 - width / 2.0, 100.0 + width / 2.0)),
        "{centred:?}"
    );
    let right = ends(Some(200.0), TextAlign::Right, "ab");
    assert!(near(right, (200.0 - width, 200.0)), "{right:?}");
    // With no width the line hangs on the anchor.
    let straddling = ends(None, TextAlign::Center, "ab");
    assert!(
        near(straddling, (-width / 2.0, width / 2.0)),
        "{straddling:?}"
    );
    let ending = ends(None, TextAlign::Right, "ab");
    assert!(near(ending, (-width, 0.0)), "{ending:?}");
    // An empty line's caret is where its text would start.
    assert_eq!(ends(Some(200.0), TextAlign::Center, ""), (100.0, 100.0));
    assert_eq!(ends(Some(200.0), TextAlign::Right, ""), (200.0, 200.0));
}

#[test]
fn the_same_text_and_spec_lay_out_the_same_every_time() {
    let measure = GlyphMeasure::new();
    let first = measure.layout("hello wörld", &spec(Some(60.0)));
    for filler in 0..40 {
        measure.layout(&format!("filler {filler}"), &spec(None));
    }
    assert_eq!(measure.layout("hello wörld", &spec(Some(60.0))), first);
    assert_eq!(
        GlyphMeasure::new().layout("hello wörld", &spec(Some(60.0))),
        first
    );
}

/// A sticky at (100, 100) reading `text`, being edited with the real
/// measure. Its text starts 8 units in.
fn editing(text: &str) -> (TestApp, TextLayout) {
    let measure = Arc::new(GlyphMeasure::new());
    let note = Entity::new(
        "n",
        Rect::new(100.0, 100.0, 200.0, 200.0),
        Kind::Text(Text {
            text: text.to_owned(),
            style: Some(TextStyle::Sticky),
            width_mode: Some(WidthMode::Fixed),
            size: Some(16.0),
            ..Text::default()
        }),
    );
    let mut app = TestApp::with_entities([note]);
    app.measure_with(measure.clone());
    app.double_click((150.0, 280.0));
    let frame = app.app().text_frame(&"n".into());
    let layout = measure.layout(text, &frame.map_or(spec(None), |frame| frame.spec));
    (app, layout)
}

/// Clicks a hair to each side of the middle of every glyph on every line,
/// and checks the caret goes to the stop on that side.
fn clicks_land_on_the_nearer_stop(text: &str) {
    let (mut app, layout) = editing(text);
    // The stops the clicks are checked against are real: inside their own
    // line, on character boundaries, and the lines stack and cover the text.
    assert_eq!(
        layout.lines.last().map(|line| line.range.end),
        Some(text.len())
    );
    for line in &layout.lines {
        for stop in &line.stops {
            assert!(
                line.range.contains(&stop.offset) || stop.offset == line.range.end,
                "{stop:?} outside {:?} in {text:?}",
                line.range
            );
            assert!(text.is_char_boundary(stop.offset), "{stop:?} in {text:?}");
        }
    }
    for pair in layout.lines.windows(2) {
        assert_eq!(pair[1].top, pair[0].top + pair[0].height, "{text:?}");
    }
    let origin = Vec2::new(108.0, 108.0);
    for line in &layout.lines {
        let y = line.top + line.height / 2.0;
        for pair in line.stops.windows(2) {
            // A wrapped line's last stop belongs to the next line.
            if pair[1].offset == line.range.end && pair[1].offset != text.len() {
                let next = layout
                    .lines
                    .iter()
                    .any(|next| next.range.start == pair[1].offset);
                if next {
                    continue;
                }
            }
            let middle = f32::midpoint(pair[0].x, pair[1].x);
            app.click(origin + Vec2::new(middle - 0.5, y));
            assert_eq!(
                app.caret().0,
                pair[0].offset,
                "left of {middle} in {text:?}"
            );
            app.click(origin + Vec2::new(middle + 0.5, y));
            assert_eq!(
                app.caret().0,
                pair[1].offset,
                "right of {middle} in {text:?}"
            );
        }
    }
}

#[test]
fn a_click_lands_on_the_nearer_side_of_a_glyph_on_any_line() {
    for text in [
        "one two three four five six seven\neight",
        "a\u{1F44D}\u{1F3FD}b\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}c",
        "日本語のテキストを折り返して二行にする",
    ] {
        clicks_land_on_the_nearer_stop(text);
    }
}

#[test]
fn the_edited_text_wraps_where_the_measure_says_and_the_note_grows_to_fit() {
    let long = "word ".repeat(60);
    let (mut app, layout) = editing(long.trim_end());
    assert!(layout.lines.len() > 8, "{} lines", layout.lines.len());
    // Typing refits the note: its padding plus every line.
    app.chord(specular_testkit::CMD, specular_interact::Key::ArrowDown)
        .type_text("x");
    let fitted = app.rect("n").height;
    let typed = format!("{}x", long.trim_end());
    let frame = app
        .app()
        .text_frame(&"n".into())
        .expect("the note is edited");
    let layout = app.app().text_measure().layout(&typed, &frame.spec);
    let lines = layout.height();
    // Every line is one line height tall.
    assert_eq!(lines, frame.spec.line_height * layout.lines.len() as f32);
    assert_eq!(fitted, f64::from((lines + 16.0).ceil()));
    assert!(fitted > 200.0);
}

#[test]
fn an_emoji_is_set_larger_than_its_em_in_small_text_and_at_its_em_in_large() {
    let measure = GlyphMeasure::new();
    let advance = |size: f32, text: &str| {
        let spec = TextSpec {
            size,
            line_height: size * 1.5,
            ..spec(None)
        };
        let layout = measure.layout(text, &spec);
        (layout.width(), layout.height())
    };
    // What CoreText measures: 21 px at 16, the em itself from 28 up.
    for emoji in ["\u{1F389}", "\u{2764}\u{FE0F}", "\u{1F1EF}\u{1F1F5}"] {
        assert_eq!(advance(16.0, emoji), (21.0, 24.0), "{emoji}");
        assert_eq!(advance(40.0, emoji), (40.0, 60.0), "{emoji}");
    }
    // The text around an emoji is set as it was.
    let (alone, _) = advance(16.0, "ab");
    let (around, _) = advance(16.0, "a\u{1F389}b");
    assert!((around - alone - 21.0).abs() < 0.01, "{around} {alone}");
}

#[test]
fn text_is_measured_the_same_with_and_without_an_emoji_free_fast_path() {
    // A run with no emoji takes the plain path, one with an emoji the rich
    // one. The letters must land where they did either way.
    let measure = GlyphMeasure::new();
    let plain = measure.layout("wrap these words", &spec(Some(60.0)));
    let rich = measure.layout("wrap these words \u{1F389}", &spec(Some(60.0)));
    assert_eq!(ranges(&plain)[0], ranges(&rich)[0]);
    assert_eq!(plain.lines[0].stops, rich.lines[0].stops);
}
