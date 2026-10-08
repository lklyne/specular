//! Lucide's glyphs: a 24 unit box, stroked 2 units wide with round caps and
//! joins in the control's text color. Each is the node list of the icon in
//! `lucide-react`, its paths joined into one.

use super::{Glyph, Layer, Paint, Shape};

/// The side of Lucide's view box.
const VIEW: f32 = 24.0;

const fn line(d: &'static str) -> Layer {
    Layer::stroke(Shape::Path(d), Paint::Current, 2.0)
}

const fn lucide(layers: &'static [Layer]) -> Glyph {
    Glyph::square(VIEW, layers)
}

/// `chevron-down`.
pub(super) const CHEVRON: Glyph = lucide(&[line("M6 9l6 6 6-6")]);
/// `chevron-left`.
pub(super) const CHEVRON_LEFT: Glyph = lucide(&[line("M15 18l-6-6 6-6")]);
/// `chevron-right`.
pub(super) const CHEVRON_RIGHT: Glyph = lucide(&[line("M9 18l6-6-6-6")]);
/// `rotate-cw`.
pub(super) const ROTATE_CW: Glyph = lucide(&[line(
    "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8M21 3v5h-5",
)]);
/// `x`.
pub(super) const X: Glyph = lucide(&[line("M18 6L6 18M6 6l12 12")]);
/// `check`.
pub(super) const CHECK: Glyph = lucide(&[line("M20 6 9 17l-5-5")]);

/// `text-align-start`.
pub(super) const ALIGN_LEFT: Glyph = lucide(&[line("M21 5H3M15 12H3M17 19H3")]);
/// `text-align-center`.
pub(super) const ALIGN_CENTER: Glyph = lucide(&[line("M21 5H3M17 12H7M19 19H5")]);
/// `text-align-end`.
pub(super) const ALIGN_RIGHT: Glyph = lucide(&[line("M21 5H3M21 12H9M21 19H7")]);
/// `ban`.
pub(super) const BAN: Glyph = lucide(&[
    Layer::stroke(Shape::Disc(12.0, 12.0, 10.0), Paint::Current, 2.0),
    line("M4.929 4.929 19.07 19.071"),
]);
/// `arrow-left`.
pub(super) const ARROW_LEFT: Glyph = lucide(&[line("M12 19l-7-7 7-7M19 12H5")]);
/// `arrow-right`.
pub(super) const ARROW_RIGHT: Glyph = lucide(&[line("M5 12h14M12 5l7 7-7 7")]);
/// `trash-2`.
pub(super) const TRASH: Glyph = lucide(&[line(
    "M10 11v6M14 11v6M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6M3 6h18\
     M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2",
)]);
/// `bold`.
pub(super) const BOLD: Glyph = lucide(&[line(
    "M6 12h9a4 4 0 0 1 0 8H7a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h7a4 4 0 0 1 0 8",
)]);
/// `strikethrough`.
pub(super) const STRIKETHROUGH: Glyph = lucide(&[line(
    "M16 4H9a3 3 0 0 0-2.83 4M14 12a4 4 0 0 1 0 8H6M4 12H20",
)]);
/// `list`.
pub(super) const LIST: Glyph = lucide(&[line("M3 5h.01M3 12h.01M3 19h.01M8 5h13M8 12h13M8 19h13")]);
/// `smartphone`.
pub(super) const SMARTPHONE: Glyph = lucide(&[
    Layer::stroke(Shape::Rect(5.0, 2.0, 14.0, 20.0, 2.0), Paint::Current, 2.0),
    line("M12 18h.01"),
]);
