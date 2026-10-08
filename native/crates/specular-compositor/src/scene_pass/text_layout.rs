//! Where a shaped run lands, and when two runs share a shaping. Pure, so
//! placement is tested without a font.

use std::hash::{Hash, Hasher};

use specular_scene::{Rect, Size, TextAlign, TextRun, VerticalAlign};

/// The rect a run's lines occupy, in the run's own units, given the
/// `measured` size of the shaped lines.
///
/// An axis with an extent (`wrap_width`, `box_height`) aligns inside it; an
/// axis without one aligns against the origin.
pub(crate) fn text_rect(run: &TextRun, measured: Size) -> Rect {
    let (x, width) = if let Some(width) = run.wrap_width {
        (run.origin.x, width)
    } else {
        let shift = match run.align {
            TextAlign::Left => 0.0,
            TextAlign::Centre => measured.width * 0.5,
            TextAlign::Right => measured.width,
        };
        (run.origin.x - shift, measured.width)
    };
    let slack = run.box_height.unwrap_or(0.0) - measured.height;
    let y = run.origin.y
        + match run.vertical_align {
            VerticalAlign::Top => 0.0,
            VerticalAlign::Middle => slack * 0.5,
            VerticalAlign::Bottom => slack,
        };
    Rect::new(x, y, width, measured.height)
}

/// Whether two runs shape and wrap to the same lines. Position, the run's own
/// colour and vertical alignment do not change the shaping. A span's colour
/// does: it is set on the shaped glyphs.
pub(crate) fn same_shaping(a: &TextRun, b: &TextRun) -> bool {
    a.text == b.text
        && a.family == b.family
        && a.size.to_bits() == b.size.to_bits()
        && a.line_height.to_bits() == b.line_height.to_bits()
        && a.wrap_width.map(f32::to_bits) == b.wrap_width.map(f32::to_bits)
        && a.overflow == b.overflow
        && a.weight == b.weight
        && a.italic == b.italic
        && a.align == b.align
        && a.spans == b.spans
}

/// A hash of exactly the fields [`same_shaping`] compares.
pub(crate) fn shaping_hash(run: &TextRun) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    run.text.hash(&mut hasher);
    run.family.hash(&mut hasher);
    run.size.to_bits().hash(&mut hasher);
    run.line_height.to_bits().hash(&mut hasher);
    run.wrap_width.map(f32::to_bits).hash(&mut hasher);
    run.overflow.hash(&mut hasher);
    run.weight.hash(&mut hasher);
    run.italic.hash(&mut hasher);
    run.align.hash(&mut hasher);
    run.spans.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use specular_scene::{Color, Point, SpanStyle, TextSpan};

    use super::*;

    const MEASURED: Size = Size::new(60.0, 20.0);

    fn run() -> TextRun {
        TextRun::new("label", Point::new(100.0, 50.0), 14.0, Color::BLACK)
    }

    #[test]
    fn an_unboxed_left_top_run_starts_at_its_origin() {
        assert_eq!(
            text_rect(&run(), MEASURED),
            Rect::new(100.0, 50.0, 60.0, 20.0)
        );
    }

    #[test]
    fn an_unboxed_centred_run_straddles_its_origin() {
        let run = TextRun {
            align: TextAlign::Centre,
            vertical_align: VerticalAlign::Middle,
            ..run()
        };
        assert_eq!(text_rect(&run, MEASURED), Rect::new(70.0, 40.0, 60.0, 20.0));
    }

    #[test]
    fn an_unboxed_right_bottom_run_ends_at_its_origin() {
        let run = TextRun {
            align: TextAlign::Right,
            vertical_align: VerticalAlign::Bottom,
            ..run()
        };
        assert_eq!(text_rect(&run, MEASURED), Rect::new(40.0, 30.0, 60.0, 20.0));
    }

    #[test]
    fn a_wrapped_run_keeps_its_box_width_whatever_the_alignment() {
        let run = TextRun {
            wrap_width: Some(200.0),
            align: TextAlign::Centre,
            ..run()
        };
        assert_eq!(
            text_rect(&run, MEASURED),
            Rect::new(100.0, 50.0, 200.0, 20.0)
        );
    }

    #[test]
    fn a_boxed_run_is_centred_in_its_height() {
        let run = TextRun {
            box_height: Some(100.0),
            vertical_align: VerticalAlign::Middle,
            ..run()
        };
        assert_eq!(text_rect(&run, MEASURED).y, 90.0);
    }

    #[test]
    fn moving_or_recolouring_a_run_keeps_its_shaping() {
        let moved = TextRun {
            origin: Point::new(1.0, 2.0),
            color: Color::WHITE,
            vertical_align: VerticalAlign::Bottom,
            ..run()
        };
        assert!(same_shaping(&run(), &moved) && shaping_hash(&run()) == shaping_hash(&moved));
    }

    #[test]
    fn a_restyled_span_is_a_new_shaping() {
        let span = |weight| TextSpan {
            range: 0..2,
            style: SpanStyle {
                weight: Some(weight),
                ..SpanStyle::default()
            },
        };
        let (bold, black) = (
            TextRun {
                spans: vec![span(700)],
                ..run()
            },
            TextRun {
                spans: vec![span(900)],
                ..run()
            },
        );
        assert!(!same_shaping(&run(), &bold) && !same_shaping(&bold, &black));
        assert_ne!(shaping_hash(&bold), shaping_hash(&black));
    }

    #[test]
    fn a_new_wrap_width_is_a_new_shaping() {
        let wrapped = TextRun {
            wrap_width: Some(80.0),
            ..run()
        };
        assert!(!same_shaping(&run(), &wrapped) && shaping_hash(&run()) != shaping_hash(&wrapped));
    }
}
