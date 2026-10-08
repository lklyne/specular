//! The measure an [`App`](crate::App) lays text out with, and the estimate
//! that stands in until the shell installs one on the renderer's fonts. The
//! trait and the layout it returns are `specular_core::text`'s.

use std::sync::Arc;

use specular_core::text::TextAlign;
pub use specular_core::text::{CaretStop, LayoutLine, TextLayout, TextMeasure, TextSpec};
use unicode_segmentation::UnicodeSegmentation;

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
