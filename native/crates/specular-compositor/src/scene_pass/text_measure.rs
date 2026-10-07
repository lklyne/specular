//! [`GlyphMeasure`]: the editor's text measure, on the fonts and the shaping
//! the scene pass draws with.
//!
//! A [`TextSpec`] becomes the [`TextRun`] `view` draws for it, and that run
//! goes through the same [`shape`] as a drawn one. The caret stops are read
//! off the shaped glyphs, in the run's own units, so they are where the
//! glyphs land at any zoom.

use std::collections::VecDeque;
use std::ops::Range;
use std::sync::{Mutex, PoisonError};

use glyphon::Buffer;
use glyphon::cosmic_text::LayoutGlyph;
use specular_doc::TextAlign;
use specular_interact::{CaretStop, LayoutLine, SourceSpan, TextLayout, TextMeasure, TextSpec};
use specular_scene::{Color, Point, TextRun};
use unicode_segmentation::UnicodeSegmentation;

use super::text_shape::shape;
use crate::fonts::Fonts;

/// Layouts kept. The editor asks for the same one several times a frame and
/// for a new one on every key.
const KEPT: usize = 32;

/// Lays text out with the renderer's fonts and shaping.
/// [`Compositor::text_measure`](crate::Compositor::text_measure) gives one
/// on the compositor's own font system. [`GlyphMeasure::new`] loads the
/// system fonts for itself, which a test or a tool with no GPU can use.
#[derive(Debug, Default)]
pub struct GlyphMeasure {
    fonts: Fonts,
    recent: Mutex<VecDeque<Kept>>,
}

/// A layout with what it is of: the text, how it is set, and the spans
/// styled over it.
type Kept = (String, TextSpec, Vec<SourceSpan>, TextLayout);

impl GlyphMeasure {
    /// A measure on its own font system. The system fonts are loaded by the
    /// first layout.
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn sharing(fonts: Fonts) -> Self {
        Self {
            fonts,
            recent: Mutex::default(),
        }
    }
}

impl TextMeasure for GlyphMeasure {
    fn layout(&self, text: &str, spec: &TextSpec) -> TextLayout {
        self.layout_styled(text, spec, &[])
    }

    fn layout_styled(&self, text: &str, spec: &TextSpec, spans: &[SourceSpan]) -> TextLayout {
        let mut recent = self.recent.lock().unwrap_or_else(PoisonError::into_inner);
        let kept = recent
            .iter()
            .position(|(kept_text, kept_spec, kept_spans, _)| {
                kept_spec == spec && kept_text == text && kept_spans == spans
            });
        if let Some(entry) = kept.and_then(|at| recent.remove(at)) {
            let layout = entry.3.clone();
            recent.push_front(entry);
            return layout;
        }
        // The colours change no glyph's place.
        let run = TextRun::source(text, spec, spans, Point::default(), [Color::BLACK; 3]);
        let layout = match self.fonts.with(|fonts| shape(fonts, &run)) {
            Some(shaped) => lines_of(&shaped.buffer, spec, shaped.size.width),
            None => unshaped(text, spec),
        };
        recent.push_front((text.to_owned(), *spec, spans.to_vec(), layout.clone()));
        recent.truncate(KEPT);
        layout
    }

    fn is_exact(&self) -> bool {
        true
    }
}

/// The layout of text whose metrics cannot be shaped: a line a paragraph,
/// with every caret stop at the left.
fn unshaped(text: &str, spec: &TextSpec) -> TextLayout {
    let height = if spec.line_height.is_finite() {
        spec.line_height.max(0.0)
    } else {
        0.0
    };
    let mut start = 0;
    let mut lines = Vec::new();
    for (index, line) in text.split('\n').enumerate() {
        let range = start..start + line.len();
        lines.push(LayoutLine {
            stops: boundaries(text, &range)
                .map(|offset| CaretStop { offset, x: 0.0 })
                .collect(),
            range,
            top: index as f32 * height,
            height,
        });
        start += line.len() + 1;
    }
    TextLayout { lines }
}

/// The grapheme cluster boundaries of `text` from `range.start` to
/// `range.end`, both included.
fn boundaries<'a>(text: &'a str, range: &Range<usize>) -> impl Iterator<Item = usize> + 'a {
    let start = range.start;
    let ends = text
        .get(range.clone())
        .unwrap_or("")
        .grapheme_indices(true)
        .map(move |(at, grapheme)| start + at + grapheme.len());
    std::iter::once(start).chain(ends)
}

/// A shaped buffer as the lines and caret stops the editor works from.
/// `width` is the width of the longest line.
fn lines_of(buffer: &Buffer, spec: &TextSpec, width: f32) -> TextLayout {
    // Glyphs are indexed inside their own paragraph.
    let mut paragraphs = Vec::with_capacity(buffer.lines.len());
    let mut at = 0;
    for line in &buffer.lines {
        paragraphs.push(at);
        at += line.text().len() + line.ending().as_str().len();
    }
    // Unwrapped lines are aligned inside the width of the longest, and their
    // stops are measured from the anchor that width is hung on.
    let (extent, anchor) = match (spec.wrap_width, spec.align) {
        (Some(wrap), _) => (wrap, 0.0),
        (None, TextAlign::Left) => (width, 0.0),
        (None, TextAlign::Center) => (width, width / 2.0),
        (None, TextAlign::Right) => (width, width),
    };
    // Where the caret sits on a line with no glyphs.
    let blank = match spec.align {
        TextAlign::Left => 0.0,
        TextAlign::Center => extent / 2.0,
        TextAlign::Right => extent,
    };
    let runs: Vec<_> = buffer.layout_runs().collect();
    let mut lines = Vec::with_capacity(runs.len().max(1));
    let mut start = 0;
    for (index, run) in runs.iter().enumerate() {
        let continues = index
            .checked_sub(1)
            .and_then(|before| runs.get(before))
            .is_some_and(|before| before.line_i == run.line_i);
        if !continues {
            start = 0;
        }
        // A wrapped line ends where the next begins, so the spaces it broke
        // at are on it and no byte falls between two lines.
        let end = match runs.get(index + 1) {
            Some(next) if next.line_i == run.line_i => {
                let first = next.glyphs.iter().map(|glyph| glyph.start).min();
                first.unwrap_or(run.text.len()).max(start)
            }
            _ => run.text.len(),
        };
        let paragraph = paragraphs.get(run.line_i).copied().unwrap_or(0);
        let stops = stops(run.text, &(start..end), run.glyphs, blank)
            .into_iter()
            .map(|(offset, x)| CaretStop {
                offset: paragraph + offset,
                x: x - anchor,
            })
            .collect();
        lines.push(LayoutLine {
            range: paragraph + start..paragraph + end,
            top: run.line_top,
            height: run.line_height,
            stops,
        });
        start = end;
    }
    if lines.is_empty() {
        lines.push(LayoutLine {
            range: 0..0,
            top: 0.0,
            height: spec.line_height,
            stops: vec![CaretStop {
                offset: 0,
                x: blank - anchor,
            }],
        });
    }
    TextLayout { lines }
}

/// The glyphs of one cluster: the bytes they stand for, and the edges the
/// caret sits at before and after them.
struct Cluster {
    range: Range<usize>,
    lead: f32,
    trail: f32,
}

/// The clusters of a line's glyphs, in text order.
fn clusters(glyphs: &[LayoutGlyph]) -> Vec<Cluster> {
    let mut clusters: Vec<Cluster> = Vec::new();
    for glyph in glyphs {
        let (left, right) = (glyph.x, glyph.x + glyph.w);
        let rtl = glyph.level.is_rtl();
        match clusters.last_mut() {
            // A cluster of several glyphs, a base and its marks, spans them.
            Some(last) if last.range == (glyph.start..glyph.end) => {
                let (low, high) = (
                    last.lead.min(last.trail).min(left),
                    last.lead.max(last.trail).max(right),
                );
                (last.lead, last.trail) = if rtl { (high, low) } else { (low, high) };
            }
            _ => clusters.push(Cluster {
                range: glyph.start..glyph.end,
                lead: if rtl { right } else { left },
                trail: if rtl { left } else { right },
            }),
        }
    }
    // Right-to-left glyphs are stored in visual order.
    clusters.sort_by_key(|cluster| cluster.range.start);
    clusters
}

/// The caret stops of the line `range` of the paragraph `text`: each
/// grapheme boundary and its x, from the line's `glyphs`. A line with no
/// glyphs has its one stop at `blank`.
fn stops(
    text: &str,
    range: &Range<usize>,
    glyphs: &[LayoutGlyph],
    blank: f32,
) -> Vec<(usize, f32)> {
    let clusters = clusters(glyphs);
    let graphemes = |part: Range<usize>| {
        text.get(part)
            .map_or(0, |part| part.graphemes(true).count())
    };
    let mut x = clusters.first().map_or(blank, |cluster| cluster.lead);
    boundaries(text, range)
        .map(|offset| {
            let after = clusters.partition_point(|cluster| cluster.range.start <= offset);
            if let Some(cluster) = after.checked_sub(1).and_then(|at| clusters.get(at)) {
                if offset < cluster.range.end {
                    // Inside a ligature the caret steps evenly across it.
                    let done = graphemes(cluster.range.start..offset);
                    let all = graphemes(cluster.range.clone()).max(1);
                    let share = done as f32 / all as f32;
                    x = cluster.lead + (cluster.trail - cluster.lead) * share;
                } else if offset == cluster.range.end {
                    x = cluster.trail;
                }
                // Past it, on a space dropped at a wrap, the caret stays put.
            }
            (offset, x)
        })
        .collect()
}
