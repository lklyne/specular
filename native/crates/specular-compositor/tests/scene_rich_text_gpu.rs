//! Styled spans, the lines that go with text, and columns of rows, through
//! `Compositor::render_scene` on an offscreen target. Glyph shapes depend on
//! the installed fonts, so these tests only ask where the ink is and what
//! colour. Each passes with a printed skip when there is no GPU adapter.

mod common;
mod ink;
mod scene_harness;

use common::{TARGET_SIZE, pixel};
use ink::{RED, RED_TEXEL, ink, rect};
use scene_harness::Harness;
use specular_scene::{
    Color, ColumnDraw, FontFamily, Item, Point, Rect, Row, RowRule, RuleHeight, SpanStyle, TextRun,
    TextSpan,
};

/// `(left, top, right, bottom)` of the pixels `is` accepts.
fn bounds_of(pixels: &[[u8; 4]], is: impl Fn([u8; 4]) -> bool) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..TARGET_SIZE {
        for x in 0..TARGET_SIZE {
            if is(pixel(pixels, x, y)) {
                let (left, top, right, bottom) = bounds.unwrap_or((x, y, x, y));
                bounds = Some((left.min(x), top.min(y), right.max(x), bottom.max(y)));
            }
        }
    }
    bounds
}

fn is_red([r, g, b, _]: [u8; 4]) -> bool {
    r > 200 && g < 60 && b < 60
}

fn is_black([r, g, b, _]: [u8; 4]) -> bool {
    r < 60 && g < 60 && b < 60
}

fn run(text: &str, spans: Vec<(std::ops::Range<usize>, SpanStyle)>) -> TextRun {
    TextRun {
        spans: spans
            .into_iter()
            .map(|(range, style)| TextSpan { range, style })
            .collect(),
        ..TextRun::new(text, Point::new(4.0, 4.0), 12.0, Color::BLACK)
    }
}

fn render(harness: &mut Harness, run: TextRun) -> Vec<[u8; 4]> {
    harness.render(vec![Item::canvas(run)])
}

#[test]
fn a_coloured_span_is_drawn_in_its_colour_beside_the_rest() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let red = SpanStyle {
        color: Some(RED),
        ..SpanStyle::default()
    };
    let pixels = render(&mut harness, run("MM MM", vec![(3..5, red)]));
    let black = bounds_of(&pixels, is_black).expect("no black glyph was drawn");
    let red = bounds_of(&pixels, is_red).expect("no red glyph was drawn");
    assert!(red.0 > black.2, "black {black:?}, red {red:?}");
}

#[test]
fn a_heavy_span_puts_down_more_ink() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let mut inked = |spans| {
        let pixels = render(&mut harness, run("MMMM", spans));
        pixels.iter().filter(|&&texel| is_black(texel)).count()
    };
    let heavy = SpanStyle {
        weight: Some(800),
        ..SpanStyle::default()
    };
    let (regular, heavy) = (inked(Vec::new()), inked(vec![(0..4, heavy)]));
    assert!(heavy > regular + regular / 10, "{regular} then {heavy}");
}

#[test]
fn a_monospace_span_gives_narrow_letters_a_full_cell() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let mut right_edge = |spans| {
        let pixels = render(&mut harness, run("iiiiM", spans));
        ink(&pixels).expect("no glyph was drawn").2
    };
    let mono = SpanStyle {
        family: Some(FontFamily::Monospace),
        ..SpanStyle::default()
    };
    let (sans, mono) = (right_edge(Vec::new()), right_edge(vec![(0..4, mono)]));
    assert!(mono > sans + 6, "sans {sans}, mono {mono}");
}

#[test]
fn underline_and_strike_are_thin_lines_under_and_through_the_text() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // No-break spaces: glyphs with width and no ink, so the line is all
    // that is drawn.
    let blank = "\u{a0}".repeat(6);
    let mut line = |style: SpanStyle| {
        let spans = vec![(0..blank.len(), style)];
        let pixels = render(&mut harness, run(&blank, spans));
        bounds_of(&pixels, is_black).expect("no line was drawn")
    };
    let under = line(SpanStyle {
        underline: true,
        ..SpanStyle::default()
    });
    let through = line(SpanStyle {
        strike: true,
        ..SpanStyle::default()
    });
    for (left, top, right, bottom) in [under, through] {
        assert!(
            right - left >= 8 && bottom - top <= 2,
            "{under:?} {through:?}"
        );
    }
    // The run's 12 unit glyphs sit on a line box from y = 4.
    assert!(
        through.1 < under.1 && under.3 < 24 && through.1 > 4,
        "{under:?} {through:?}"
    );
}

#[test]
fn a_line_takes_its_spans_colour() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let blank = "\u{a0}".repeat(6);
    let style = SpanStyle {
        color: Some(RED),
        underline: true,
        ..SpanStyle::default()
    };
    let pixels = render(&mut harness, run(&blank, vec![(0..blank.len(), style)]));
    assert!(bounds_of(&pixels, is_red).is_some() && bounds_of(&pixels, is_black).is_none());
}

/// A 16 unit letter on a 20 unit line, then a red rule row 4 units tall
/// under a 6 unit gap.
fn column(scroll: f32, height: f32) -> ColumnDraw {
    let letter = TextRun {
        line_height: 20.0,
        ..TextRun::new("M", Point::new(0.0, 0.0), 16.0, Color::BLACK)
    };
    let rule = RowRule {
        x: 10.0,
        width: 30.0,
        height: RuleHeight::Row,
        color: RED,
    };
    ColumnDraw {
        origin: Point::new(4.0, 4.0),
        width: 56.0,
        height,
        scroll,
        rows: vec![
            Row {
                cells: vec![letter],
                ..Row::default()
            },
            Row {
                gap: 6.0,
                min_height: 4.0,
                rules: vec![rule],
                ..Row::default()
            },
        ],
        owner: None,
    }
}

#[test]
fn a_columns_rows_stack_under_the_measured_text() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let pixels = harness.render(vec![Item::canvas(column(0.0, 56.0))]);
    let letter = bounds_of(&pixels, is_black).expect("no glyph was drawn");
    // The rule starts 20 + 6 units under the column's top at y = 4.
    assert_eq!(bounds_of(&pixels, is_red), Some((14, 30, 43, 33)));
    assert!(letter.1 >= 4 && letter.3 < 24, "{letter:?}");
}

#[test]
fn a_column_keeps_its_place_in_the_paint_order() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let cover = || Item::canvas(rect(0.0, 0.0, 64.0, 64.0, RED));
    let covered = harness.render(vec![Item::canvas(column(0.0, 56.0)), cover()]);
    let on_top = harness.render(vec![cover(), Item::canvas(column(0.0, 56.0))]);
    let all_red = |pixels: &[[u8; 4]]| pixels.iter().all(|&texel| texel == RED_TEXEL);
    assert!(all_red(&covered) && !all_red(&on_top));
}

#[test]
fn a_scrolled_column_moves_up_and_stops_at_its_end() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    // 30 units of rows in a 20 unit window: 10 is as far as they go.
    for (scroll, top) in [(6.0, 24), (500.0, 20)] {
        let pixels = harness.render(vec![Item::canvas(column(scroll, 20.0))]);
        let rule = bounds_of(&pixels, is_red).expect("no rule was drawn");
        assert_eq!((rule.1, rule.3), (top, top + 3), "scroll {scroll}");
    }
}

#[test]
fn a_clipped_column_draws_nothing_outside_its_clip() {
    let Some(mut harness) = Harness::new() else {
        return;
    };
    let clip = Rect::new(0.0, 0.0, 20.0, 32.0);
    let pixels = harness.render(vec![Item::canvas(column(0.0, 56.0)).clipped(clip)]);
    let (_, _, right, bottom) = ink(&pixels).expect("nothing was drawn");
    assert!(right < 20 && bottom < 32, "{right} {bottom}");
}
