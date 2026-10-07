//! Text runs. The scene says what to set and where; shaping, wrapping and
//! glyph rasterising belong to the renderer.

use crate::{Color, Point};

/// Which typeface a run is set in.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum FontFamily {
    /// The platform's sans-serif UI face.
    #[default]
    SansSerif,
    /// The platform's serif face.
    Serif,
    /// The platform's monospace face.
    Monospace,
    /// A family by name, falling back to sans-serif when it is not installed.
    Named(String),
}

/// Horizontal placement of a run's lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextAlign {
    /// Lines start at the left.
    #[default]
    Left,
    /// Lines are centred.
    Centre,
    /// Lines end at the right.
    Right,
}

/// Vertical placement of a run's block of lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VerticalAlign {
    /// The block starts at the top.
    #[default]
    Top,
    /// The block is centred.
    Middle,
    /// The block ends at the bottom.
    Bottom,
}

/// One block of text in a single style.
///
/// `origin`, `wrap_width` and `box_height` describe a layout box. An axis
/// with an extent aligns the text inside that extent. An axis without one
/// aligns the text against `origin` itself: `Left`/`Top` start there,
/// `Centre`/`Middle` straddle it, `Right`/`Bottom` end there. A centred badge
/// label is an origin with no extents; a sticky note's text is a full box.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// The text. Line breaks in it are kept.
    pub text: String,
    /// Top-left of the layout box, or the anchor on an axis with no extent.
    pub origin: Point,
    /// Width to wrap lines at; `None` never wraps.
    pub wrap_width: Option<f32>,
    /// Height of the layout box, used only for vertical alignment. Text
    /// taller than this overflows; clip the item to cut it off.
    pub box_height: Option<f32>,
    /// Typeface.
    pub family: FontFamily,
    /// Font size, in the item's space.
    pub size: f32,
    /// Distance between baselines, in the item's space.
    pub line_height: f32,
    /// Font weight on the CSS scale (400 regular, 700 bold).
    pub weight: u16,
    /// Whether the run is italic.
    pub italic: bool,
    /// Text colour.
    pub color: Color,
    /// Horizontal alignment.
    pub align: TextAlign,
    /// Vertical alignment.
    pub vertical_align: VerticalAlign,
}

impl TextRun {
    /// Line height used by [`new`](Self::new), as a multiple of the size.
    pub const DEFAULT_LINE_HEIGHT: f32 = 1.4;

    /// A regular sans-serif run starting at `origin`, unwrapped.
    pub fn new(text: impl Into<String>, origin: Point, size: f32, color: Color) -> Self {
        Self {
            text: text.into(),
            origin,
            wrap_width: None,
            box_height: None,
            family: FontFamily::SansSerif,
            size,
            line_height: size * Self::DEFAULT_LINE_HEIGHT,
            weight: 400,
            italic: false,
            color,
            align: TextAlign::Left,
            vertical_align: VerticalAlign::Top,
        }
    }
}
