//! The toolbar's tool glyphs: `shared/icons/toolbar/*.svg` and the inline
//! ones of `shared/CustomIcons.tsx`, each in a 20 unit box.

use super::paths_pens::{
    MARKER_BODY, MARKER_SHINE, MARKER_TIP, PEN_BODY, PEN_BODY_LINE, PEN_SHINE, PEN_TIP,
    PEN_TIP_LINE,
};
use super::paths_tools::{
    COMMENT_BUBBLE, COMMENT_CORNER, DOCUMENT_BODY, DOCUMENT_LINE, SELECT_BODY, SELECT_LINE,
    SHAPE_ARROW, STICKY_CURL, STICKY_FRONT, STICKY_FRONT_LINE, TEXT,
};
use super::{BODY, Glyph, Layer, OUTLINE, Paint, Shape, Turn};

/// The line of the toolbar pen and highlighter.
const PEN_LINE: Paint = Paint::Hex(0x18_181b);
const MARKER_LINE: Paint = Paint::Hex(0x35_2c24);
/// A pen's barrel, the shade down one side of it and the seam between.
pub(super) const BARREL: Paint = Paint::Hex(0xf4_f4f4);
pub(super) const SHINE: Paint = Paint::HexA(0xdb_dbdb, 110);
pub(super) const SEAM: Paint = Paint::HexA(0xb5_b5b5, 170);

const SELECT: &[Layer] = &[
    Layer::fill(Shape::Path(SELECT_BODY), BODY),
    Layer::fill(Shape::Path(SELECT_LINE), OUTLINE),
];

/// A browser window with a phone leaning on it. The window's title bar is
/// its top corners and a line under them; the phone is turned 15 degrees
/// about its own corner.
const PAGE: &[Layer] = &[
    Layer::fill(Shape::Rect(0.5, 1.0, 16.0, 12.0, 2.0), BODY),
    Layer::fill(
        Shape::Path("M0.5 6V3a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v3z"),
        Paint::Hex(0xed_ebe3),
    ),
    Layer::stroke(Shape::Path("M0.5 6h16"), OUTLINE, 1.0),
    Layer::fill(Shape::Disc(3.5, 3.5, 1.0), OUTLINE),
    Layer::fill(Shape::Disc(6.5, 3.5, 1.0), OUTLINE),
    Layer::fill(Shape::Disc(9.5, 3.5, 1.0), OUTLINE),
    Layer::stroke(Shape::Rect(0.5, 1.0, 16.0, 12.0, 2.0), OUTLINE, 1.0),
    Layer::new(
        Shape::Rect(11.123, 3.0, 9.0, 14.0, 2.326_21),
        BODY,
        OUTLINE,
        1.0,
    )
    .turned(Turn::Rotate(15.0, 11.123, 3.0)),
    Layer::stroke(
        Shape::Path("M10.5616 14.7548L14.4253 15.7901"),
        OUTLINE,
        1.0,
    ),
];

const TEXT_TOOL: &[Layer] = &[Layer::new(Shape::Path(TEXT), BODY, OUTLINE, 1.0)];

/// Two sheets in the sticky color, the back one askew.
const STICKY: &[Layer] = &[
    Layer::fill(Shape::Rect(0.0, 0.0, 15.0, 15.0, 3.0), Paint::Paper).turned(Turn::Matrix([
        0.998_628,
        0.052_367_9,
        -0.041_698_9,
        0.999_13,
        4.401_37,
        3.613_28,
    ])),
    Layer::stroke(
        Shape::Rect(0.478_464, 0.525_749, 14.0, 14.0, 2.5),
        OUTLINE,
        1.0,
    )
    .turned(Turn::Matrix([
        0.998_628,
        0.052_367_9,
        -0.041_698_9,
        0.999_13,
        4.423_95,
        3.588_68,
    ])),
    Layer::fill(Shape::Path(STICKY_FRONT), Paint::Paper),
    Layer::stroke(Shape::Path(STICKY_FRONT_LINE), OUTLINE, 1.0),
    Layer::stroke(Shape::Path(STICKY_CURL), OUTLINE, 1.0),
];

const DOCUMENT: &[Layer] = &[
    Layer::fill(Shape::Path(DOCUMENT_LINE), OUTLINE),
    Layer::fill(Shape::Path(DOCUMENT_BODY), BODY),
];

/// A square and a circle in the shape color, and an arrow from one to the
/// other.
const SHAPE: &[Layer] = &[
    Layer::new(
        Shape::Rect(0.5, 1.5, 10.0, 10.0, 1.5),
        Paint::Paper,
        OUTLINE,
        1.0,
    ),
    Layer::new(
        Shape::Rect(6.5, 8.5, 9.0, 9.0, 4.5),
        Paint::Paper,
        OUTLINE,
        1.0,
    ),
    Layer::fill(Shape::Path(SHAPE_ARROW), OUTLINE),
];

const PEN: &[Layer] = &[
    Layer::fill(Shape::Path(PEN_TIP), Paint::Tint),
    Layer::stroke(Shape::Path(PEN_TIP_LINE), PEN_LINE, 1.0),
    Layer::fill(Shape::Path(PEN_BODY), BARREL),
    Layer::stroke(Shape::Path(PEN_BODY_LINE), PEN_LINE, 1.0),
    Layer::fill(Shape::Path(PEN_SHINE), SHINE),
    Layer::fill(Shape::Rect(10.0, 7.0, 1.0, 12.0, 0.0), SEAM),
];

const MARKER: &[Layer] = &[
    Layer::new(Shape::Path(MARKER_TIP), Paint::Tint, MARKER_LINE, 1.0),
    Layer::new(Shape::Path(MARKER_BODY), BARREL, MARKER_LINE, 1.0),
    Layer::fill(Shape::Path(MARKER_SHINE), SHINE),
    Layer::fill(Shape::Rect(5.0, 16.0, 11.0, 1.0, 0.0), SEAM),
    Layer::fill(Shape::Rect(5.0, 17.0, 11.0, 4.0, 0.0), Paint::Tint),
];

const COMMENT: &[Layer] = &[
    Layer::new(Shape::Path(COMMENT_BUBBLE), BODY, OUTLINE, 1.0),
    Layer::fill(Shape::Path(COMMENT_CORNER), OUTLINE),
];

const fn clipped(layers: &'static [Layer]) -> Glyph {
    Glyph {
        clipped: true,
        ..Glyph::square(20.0, layers)
    }
}

/// `select.svg`.
pub(super) const SELECT_TOOL: Glyph = Glyph::square(20.0, SELECT);
/// `AddPageToolIcon`.
pub(super) const PAGE_TOOL: Glyph = Glyph::square(20.0, PAGE);
/// `add-text.svg`.
pub(super) const TEXT_GLYPH: Glyph = Glyph::square(20.0, TEXT_TOOL);
/// `AddStickyToolIcon`.
pub(super) const STICKY_TOOL: Glyph = Glyph::square(20.0, STICKY);
/// `add-document.svg`, whose view box starts off the origin.
pub(super) const DOCUMENT_TOOL: Glyph = Glyph {
    view: [6.0, 4.0, 20.0, 20.0],
    ..Glyph::square(20.0, DOCUMENT)
};
/// `AddShapeToolIcon`.
pub(super) const SHAPE_TOOL: Glyph = Glyph::square(20.0, SHAPE);
/// `DrawPenToolIcon`.
pub(super) const PEN_TOOL: Glyph = clipped(PEN);
/// `DrawHighlightToolIcon`.
pub(super) const MARKER_TOOL: Glyph = clipped(MARKER);
/// `CommentToolIcon`.
pub(super) const COMMENT_TOOL: Glyph = Glyph::square(18.0, COMMENT);
