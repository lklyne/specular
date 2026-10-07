//! Text runs. The scene says what to set and where; shaping, wrapping and
//! glyph rasterising belong to the renderer.

use std::ops::Range;

use specular_doc::TextFont;
use specular_interact::{SourceSpan, TextFrame, TextSpec};

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

/// How a stretch of a run differs from the run's own style. A field left
/// `None` keeps the run's value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SpanStyle {
    /// Typeface.
    pub family: Option<FontFamily>,
    /// Font weight on the CSS scale.
    pub weight: Option<u16>,
    /// Whether the stretch is italic.
    pub italic: Option<bool>,
    /// Text colour. The item's opacity fades the run's own colour and the
    /// lines under and through a span, but not a span's glyphs.
    pub color: Option<Color>,
    /// A line under the text, in the text's colour.
    pub underline: bool,
    /// A line through the text, in the text's colour.
    pub strike: bool,
}

/// A stretch of a run set in its own [`SpanStyle`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TextSpan {
    /// The bytes of the run's text it covers.
    pub range: Range<usize>,
    /// How it is set.
    pub style: SpanStyle,
}

/// One block of text. It has one style, which [`spans`](Self::spans) can
/// override for parts of it.
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
    /// The stretches set differently from the rest, in text order and not
    /// overlapping. A span that overlaps the one before it, runs past the
    /// text or cuts a character in two is ignored.
    pub spans: Vec<TextSpan>,
}

impl TextRun {
    /// Line height used by [`new`](Self::new), as a multiple of the size.
    pub const DEFAULT_LINE_HEIGHT: f32 = 1.4;
    /// Weight of headings and strong text in a Document.
    pub const HEAVY: u16 = 600;

    /// `text` set as `spec` says, starting at `origin`. `view` draws an
    /// entity's text with this run and the editor's measure shapes the same
    /// one, so a caret measured on it sits on the drawn glyphs.
    pub fn set(text: impl Into<String>, spec: &TextSpec, origin: Point, color: Color) -> Self {
        Self {
            wrap_width: spec.wrap_width,
            family: match spec.font {
                TextFont::Sans => FontFamily::SansSerif,
                TextFont::Mono => FontFamily::Monospace,
                TextFont::Hand => FontFamily::Named("Kalam".to_owned()),
            },
            line_height: spec.line_height,
            align: match spec.align {
                specular_doc::TextAlign::Left => TextAlign::Left,
                specular_doc::TextAlign::Center => TextAlign::Centre,
                specular_doc::TextAlign::Right => TextAlign::Right,
            },
            ..Self::new(text, origin, spec.size, color)
        }
    }

    /// One line of markdown source set as `spec` says with `spans` styled
    /// over it, starting at `origin`. A Document being edited is drawn in
    /// these and the editor's measure shapes the same ones. `faint` is the
    /// colour of syntax and `link` of a link's text; neither changes where
    /// a glyph lands.
    pub fn source(
        text: impl Into<String>,
        spec: &TextSpec,
        spans: &[SourceSpan],
        origin: Point,
        [ink, faint, link]: [Color; 3],
    ) -> Self {
        let spans = spans
            .iter()
            .map(|span| {
                let style = span.style;
                TextSpan {
                    range: span.range.clone(),
                    style: SpanStyle {
                        family: style.code.then_some(FontFamily::Monospace),
                        weight: style.strong.then_some(Self::HEAVY),
                        italic: style.emphasis.then_some(true),
                        color: if style.faint {
                            Some(faint)
                        } else if style.link {
                            Some(link)
                        } else {
                            None
                        },
                        underline: style.link && !style.faint,
                        strike: style.strike && !style.faint,
                    },
                }
            })
            .collect();
        Self {
            spans,
            ..Self::set(text, spec, origin, ink)
        }
    }

    /// `text` laid out in `frame`, the place an entity's text sits.
    pub fn framed(text: impl Into<String>, frame: &TextFrame, color: Color) -> Self {
        let origin = Point::new(frame.origin.x as f32, frame.origin.y as f32);
        Self {
            box_height: frame.box_height,
            vertical_align: match frame.vertical {
                specular_doc::VerticalAlign::Top => VerticalAlign::Top,
                specular_doc::VerticalAlign::Middle => VerticalAlign::Middle,
                specular_doc::VerticalAlign::Bottom => VerticalAlign::Bottom,
            },
            ..Self::set(text, &frame.spec, origin, color)
        }
    }

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
            spans: Vec::new(),
        }
    }
}
