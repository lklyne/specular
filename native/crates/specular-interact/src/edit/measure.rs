//! [`TextMeasure`]: the one question the editor asks about fonts.
//!
//! Everything that depends on where glyphs land (up and down, the ends of a
//! wrapped line, a click, the caret and selection rects, a text entity's
//! height as it grows) is computed here from a [`TextLayout`], which is plain
//! data. The trait only has to produce one.

use std::fmt::Debug;
use std::ops::Range;
use std::sync::Arc;

use specular_doc::{TextAlign, TextFont};
use unicode_segmentation::UnicodeSegmentation;

use super::source::SourceSpan;

/// How a block of text is set: what a [`TextMeasure`] needs besides the
/// text. Lengths are in canvas units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSpec {
    /// Typeface.
    pub font: TextFont,
    /// Font size.
    pub size: f32,
    /// Distance between baselines.
    pub line_height: f32,
    /// Width to wrap lines at; `None` never wraps.
    pub wrap_width: Option<f32>,
    /// Horizontal alignment of each line.
    pub align: TextAlign,
}

/// Lays text out. The shell installs one that shapes with the renderer's
/// fonts, so the caret sits where the glyphs are drawn.
///
/// An implementation must be a pure function of its arguments: `update`
/// calls it, and `update` has to give the same answer every time.
pub trait TextMeasure: Debug + Send + Sync {
    /// Where every line and caret position of `text` falls when set as
    /// `spec` says. See [`TextLayout`] for what must hold.
    fn layout(&self, text: &str, spec: &TextSpec) -> TextLayout;

    /// The layout of one line of markdown source set with `spans` styled
    /// over it: heavy, italic and monospace stretches are as wide as they
    /// are drawn. An estimate that knows no fonts can leave this as it is.
    fn layout_styled(&self, text: &str, spec: &TextSpec, spans: &[SourceSpan]) -> TextLayout {
        let _ = spans;
        self.layout(text, spec)
    }

    /// Whether the layouts are what the renderer draws. A document's text
    /// entities are given their measured size on load only when they are:
    /// an estimate knows less than the heights the file carries.
    fn is_exact(&self) -> bool {
        false
    }
}

/// `text` as laid-out lines.
///
/// What an implementation must keep to:
/// - There is at least one line, even for empty text, and text ending in a
///   line break has an empty last line.
/// - Lines run top to bottom and their ranges are in text order. A line
///   break character is in no line's range. A line that was wrapped ends at
///   the byte the next line starts at.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextLayout {
    /// The visual lines.
    pub lines: Vec<LayoutLine>,
}

/// One visual line of a [`TextLayout`].
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutLine {
    /// The bytes of the text on this line.
    pub range: Range<usize>,
    /// The top of the line box, measured down from the top of the first
    /// line.
    pub top: f32,
    /// The height of the line box.
    pub height: f32,
    /// Every place the caret can sit on this line: one per grapheme cluster
    /// boundary from `range.start` to `range.end` inclusive, in byte order.
    pub stops: Vec<CaretStop>,
}

/// One caret position on a line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretStop {
    /// The byte offset into the text.
    pub offset: usize,
    /// Where the caret is drawn. With a wrap width this is measured from the
    /// left edge of the layout box, alignment included. With none it is
    /// measured from the anchor: left-aligned text starts at 0, centred text
    /// straddles 0 and right-aligned text ends at 0.
    pub x: f32,
}

/// The measure an [`App`](crate::App) uses, which is an estimate until the
/// shell installs a real one.
#[derive(Debug, Clone)]
pub(crate) struct Measurer(pub(crate) Arc<dyn TextMeasure>);

impl Default for Measurer {
    fn default() -> Self {
        Self(Arc::new(Estimate))
    }
}

/// Stands in until a real measure is installed: every grapheme is half the
/// font size wide and nothing wraps.
#[derive(Debug, Clone, Copy)]
struct Estimate;

impl TextMeasure for Estimate {
    fn layout(&self, text: &str, spec: &TextSpec) -> TextLayout {
        let advance = spec.size * 0.5;
        let mut lines = Vec::new();
        let mut start = 0;
        for (index, line) in text.split('\n').enumerate() {
            let width = line.graphemes(true).count() as f32 * advance;
            let left = match (spec.align, spec.wrap_width) {
                (TextAlign::Left, _) => 0.0,
                (TextAlign::Center, extent) => (extent.unwrap_or(0.0) - width) / 2.0,
                (TextAlign::Right, extent) => extent.unwrap_or(0.0) - width,
            };
            let ends = line
                .grapheme_indices(true)
                .map(|(at, grapheme)| at + grapheme.len());
            let stops = std::iter::once(0)
                .chain(ends)
                .enumerate()
                .map(|(count, at)| CaretStop {
                    offset: start + at,
                    x: left + count as f32 * advance,
                })
                .collect();
            lines.push(LayoutLine {
                range: start..start + line.len(),
                top: index as f32 * spec.line_height,
                height: spec.line_height,
                stops,
            });
            start += line.len() + 1;
        }
        TextLayout { lines }
    }
}
