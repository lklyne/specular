//! The popup's own glyphs: the pens and stroke samples of
//! `shared/CustomIcons.tsx`, the line glyphs of `BorderDropdown.tsx` and
//! `EdgeStrokeDropdown.tsx`, the color scheme assets, and the shape
//! silhouettes of `src/shared/shapes.ts` as `ShapeGlyph.tsx` draws them.

use specular_doc::ShapeKind;

use super::paths_pens::{SLIM_BODY, SLIM_SHINE, SLIM_TIP, WIDE_BODY, WIDE_SHINE, WIDE_TIP};
use super::paths_popup::{
    BORDER_TOP, MOON_BODY, MOON_LINE, ROTATE_BOTTOM, ROTATE_TOP, STROKE_THICK, STROKE_THIN,
    SUN_LINE, SYSTEM_LINE, SYSTEM_MOON,
};
use super::tools::{BARREL, SEAM, SHINE};
use super::{BODY, Glyph, Layer, OUTLINE, Paint, Shape};

/// The color of a stroke sample: `#3f3f46` in `StrokeWidthSwatch.tsx`.
const SAMPLE: Paint = Paint::Hex(0x3f_3f46);

const fn clipped(layers: &'static [Layer]) -> Glyph {
    Glyph {
        clipped: true,
        ..Glyph::square(16.0, layers)
    }
}

/// `PenSlimIcon`.
pub(super) const BRUSH_PEN: Glyph = clipped(&[
    Layer::new(Shape::Path(SLIM_TIP), Paint::Tint, Paint::PenLine, 0.75),
    Layer::new(Shape::Path(SLIM_BODY), BARREL, Paint::PenLine, 0.75),
    Layer::fill(Shape::Path(SLIM_SHINE), SHINE),
    Layer::fill(Shape::Rect(7.25, 6.25, 0.75, 9.0, 0.0), SEAM),
]);
/// `PenMarkerIcon`.
pub(super) const BRUSH_HIGHLIGHTER: Glyph = clipped(&[
    Layer::new(Shape::Path(WIDE_TIP), Paint::Tint, Paint::Tint, 1.0),
    Layer::new(Shape::Path(WIDE_BODY), BARREL, Paint::PenLine, 1.0),
    Layer::fill(Shape::Path(WIDE_SHINE), SHINE),
    Layer::fill(Shape::Rect(4.4004, 13.0, 8.0, 1.0, 0.0), SEAM),
    Layer::fill(Shape::Rect(4.0, 13.5996, 8.8, 3.2, 0.0), Paint::Tint),
]);
/// `StrokeThinIcon`, 17 by 9.
pub(super) const STROKE_THIN_SAMPLE: Glyph = Glyph {
    view: [0.0, 0.0, 17.0, 9.0],
    clipped: false,
    layers: &[Layer::stroke(Shape::Path(STROKE_THIN), SAMPLE, 1.0)],
};
/// `StrokeThickIcon`, 19 by 11.
pub(super) const STROKE_THICK_SAMPLE: Glyph = Glyph {
    view: [0.0, 0.0, 19.0, 11.0],
    clipped: false,
    layers: &[Layer::stroke(Shape::Path(STROKE_THICK), SAMPLE, 3.0)],
};
/// `BorderGlyph`: three lines of growing weight.
pub(super) const BORDER: Glyph = Glyph::square(
    14.0,
    &[
        Layer::fill(Shape::Path(BORDER_TOP), Paint::Current),
        Layer::stroke(Shape::Rect(1.5, 5.0, 11.0, 2.0, 0.5), Paint::Current, 1.0),
        Layer::stroke(Shape::Rect(1.5, 9.0, 11.0, 3.0, 0.5), Paint::Current, 1.0),
    ],
);
/// `LineGlyph`, solid.
pub(super) const LINE_SOLID: Glyph = Glyph::square(
    14.0,
    &[Layer::stroke(Shape::Path("M1 7H13"), Paint::Current, 2.0)],
);
/// `LineGlyph`, dashed: three short bars.
pub(super) const LINE_DASHED: Glyph = Glyph::square(
    14.0,
    &[
        Layer::fill(Shape::Rect(0.5, 6.0, 3.0, 2.0, 1.0), Paint::Current),
        Layer::fill(Shape::Rect(5.5, 6.0, 3.0, 2.0, 1.0), Paint::Current),
        Layer::fill(Shape::Rect(10.5, 6.0, 3.0, 2.0, 1.0), Paint::Current),
    ],
);
/// `RotateIcon`.
pub(super) const ROTATE: Glyph = Glyph::square(
    14.0,
    &[
        Layer::fill(Shape::Path(ROTATE_TOP), Paint::Current),
        Layer::fill(Shape::Path(ROTATE_BOTTOM), Paint::Current),
    ],
);
/// `sun-moon.svg`.
pub(super) const SCHEME_SYSTEM: Glyph = Glyph::square(
    20.0,
    &[
        Layer::fill(Shape::Path(SYSTEM_LINE), OUTLINE),
        Layer::fill(Shape::Path(SYSTEM_MOON), BODY),
    ],
);
/// `sun.svg`, whose view box starts off the origin.
pub(super) const SCHEME_LIGHT: Glyph = Glyph {
    view: [2.8333, 0.8333, 20.0, 20.0],
    clipped: false,
    layers: &[
        Layer::fill(Shape::Path(SUN_LINE), OUTLINE),
        Layer::new(Shape::Disc(12.8333, 10.8333, 3.5), BODY, OUTLINE, 1.0),
    ],
};
/// `moon.svg`, whose view box starts off the origin.
pub(super) const SCHEME_DARK: Glyph = Glyph {
    view: [1.0, 0.0, 20.0, 20.0],
    clipped: false,
    layers: &[
        Layer::fill(Shape::Path(MOON_BODY), BODY),
        Layer::fill(Shape::Path(MOON_LINE), OUTLINE),
    ],
};

const fn silhouette(d: &'static str) -> Layer {
    Layer::stroke(Shape::Path(d), Paint::Current, 8.0)
}

/// The silhouette of a shape kind: its path in a 100 unit box, stroked 8
/// units wide, in a view box padded by 6 so the stroke is not cut off.
pub(super) const fn shape(kind: ShapeKind) -> Glyph {
    let layers: &'static [Layer] = match kind {
        ShapeKind::Rectangle => const { &[silhouette("M0,0 H100 V100 H0 Z")] },
        ShapeKind::Rounded => {
            const {
                &[silhouette(
                    "M15,0 H85 A15,15 0 0 1 100,15 V85 A15,15 0 0 1 85,100 H15 A15,15 0 0 1 0,85 \
                 V15 A15,15 0 0 1 15,0 Z",
                )]
            }
        }
        ShapeKind::Ellipse => {
            const { &[silhouette("M0,50 A50,50 0 1 1 100,50 A50,50 0 1 1 0,50 Z")] }
        }
        ShapeKind::Diamond => const { &[silhouette("M50,0 L100,50 L50,100 L0,50 Z")] },
        ShapeKind::Triangle => const { &[silhouette("M50,6.7 L100,93.3 L0,93.3 Z")] },
        ShapeKind::Hexagon => const { &[silhouette("M25,6.7 H75 L100,50 L75,93.3 H25 L0,50 Z")] },
        ShapeKind::Pill => {
            const {
                &[silhouette(
                    "M30,20 H70 A30,30 0 0 1 70,80 H30 A30,30 0 0 1 30,20 Z",
                )]
            }
        }
        ShapeKind::Parallelogram => const { &[silhouette("M22,0 H100 L78,100 H0 Z")] },
        ShapeKind::Chevron => const { &[silhouette("M0,0 H70 L100,50 L70,100 H0 L30,50 Z")] },
        ShapeKind::Cylinder => {
            const {
                &[
                    silhouette("M0,15 A50,15 0 0 1 100,15 L100,85 A50,15 0 0 1 0,85 Z"),
                    silhouette("M0,15 A50,15 0 0 0 100,15"),
                ]
            }
        }
    };
    Glyph {
        view: [-6.0, -6.0, 112.0, 112.0],
        clipped: false,
        layers,
    }
}
