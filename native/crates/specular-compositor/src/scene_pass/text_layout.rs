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
