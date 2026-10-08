//! Rects and ellipses, the shapes the renderer draws from a distance field.

use crate::{Color, Rect};

/// Where a shape's stroke sits relative to its edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StrokeAlign {
    /// Wholly inside the edge, like a CSS border. The fill shows under it.
    #[default]
    Inside,
    /// Half on each side of the edge.
    Centre,
    /// Wholly outside the edge, so an outline never covers what it frames.
    Outside,
}

/// The outline of a rect or ellipse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    /// Stroke colour.
    pub color: Color,
    /// Stroke width, in the item's space.
    pub width: f32,
    /// Which side of the edge the stroke sits on.
    pub align: StrokeAlign,
}

impl Stroke {
    /// A stroke of `width`.
    pub const fn new(color: Color, width: f32, align: StrokeAlign) -> Self {
        Self {
            color,
            width,
            align,
        }
    }

    /// How far the stroke reaches past the shape's edge.
    pub fn outset(self) -> f32 {
        let width = self.width.max(0.0);
        match self.align {
            StrokeAlign::Inside => 0.0,
            StrokeAlign::Centre => width * 0.5,
            StrokeAlign::Outside => width,
        }
    }
}

/// A filled and/or stroked rectangle with rounded corners.
///
/// Sticky notes, group frames, page borders, selection outlines, resize
/// handles, the marquee and comment regions are all this.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectDraw {
    /// Position and size.
    pub rect: Rect,
    /// Corner radius, clamped by the renderer to half the shorter side.
    pub corner_radius: f32,
    /// Fill colour, if filled.
    pub fill: Option<Color>,
    /// Outline, if stroked.
    pub stroke: Option<Stroke>,
}

impl RectDraw {
    /// A filled rect with square corners and no stroke.
    pub const fn filled(rect: Rect, fill: Color) -> Self {
        Self {
            rect,
            corner_radius: 0.0,
            fill: Some(fill),
            stroke: None,
        }
    }

    /// An unfilled rect with square corners.
    pub const fn outlined(rect: Rect, stroke: Stroke) -> Self {
        Self {
            rect,
            corner_radius: 0.0,
            fill: None,
            stroke: Some(stroke),
        }
    }

    /// The same rect with rounded corners.
    #[must_use]
    pub const fn with_corner_radius(self, corner_radius: f32) -> Self {
        Self {
            corner_radius,
            ..self
        }
    }

    /// The same rect with an outline.
    #[must_use]
    pub const fn with_stroke(self, stroke: Stroke) -> Self {
        Self {
            stroke: Some(stroke),
            ..self
        }
    }
}

/// The soft shadow a rounded rect casts, as a CSS `box-shadow` draws it:
/// the rect's own shape, blurred. It goes in the list before the rect that
/// casts it, already moved by the shadow's offset.
///
/// The renderer draws it from the same distance field as a rect, one
/// instance in the same batch, so a shadow costs what a second fill would
/// and nothing is blurred.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowDraw {
    /// The caster's rect, moved by the shadow's offset.
    pub rect: Rect,
    /// The caster's corner radius.
    pub corner_radius: f32,
    /// The blur radius, as CSS counts it: the shadow fades from full to
    /// nothing across about twice this, centred on the rect's edge.
    pub blur: f32,
    /// The shadow's colour where it is densest.
    pub color: Color,
}

/// A filled and/or stroked ellipse inscribed in `rect`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipseDraw {
    /// The ellipse's bounding box.
    pub rect: Rect,
    /// Fill colour, if filled.
    pub fill: Option<Color>,
    /// Outline, if stroked.
    pub stroke: Option<Stroke>,
}

impl EllipseDraw {
    /// A filled ellipse with no stroke.
    pub const fn filled(rect: Rect, fill: Color) -> Self {
        Self {
            rect,
            fill: Some(fill),
            stroke: None,
        }
    }

    /// The same ellipse with an outline.
    #[must_use]
    pub const fn with_stroke(self, stroke: Stroke) -> Self {
        Self {
            stroke: Some(stroke),
            ..self
        }
    }
}
