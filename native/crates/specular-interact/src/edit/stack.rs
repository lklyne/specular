//! A Document's source laid out for editing: one row per source line,
//! stacked top to bottom, each set in the size its syntax gives it.
//!
//! [`source_rows`] is the one description of those rows. The editor measures
//! them here into a single [`TextLayout`], so every caret and selection
//! query works on a Document as it does on a sticky, and `view` draws the
//! same rows, so the caret sits on the glyphs.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, Mutex, PoisonError};

use super::measure::{LayoutLine, TextLayout, TextMeasure, TextSpec};
use super::source::{self, SourceSpan};

/// Sizes of the first three heading levels as multiples of the body size.
/// Deeper headings are body size.
const HEADING_SCALE: [f32; 3] = [1.4, 1.2, 1.1];

/// One source line of a Document being edited.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceRow {
    /// The bytes of the text the row holds, without its line break.
    pub range: Range<usize>,
    /// How the row is set: the body's spec, at a heading's size on a heading.
    pub spec: TextSpec,
    /// The stretches styled over it, in bytes from the row's start.
    pub spans: Vec<SourceSpan>,
}

/// The rows of `text` when its body text is set as `body` says.
pub fn source_rows(text: &str, body: &TextSpec) -> Vec<SourceRow> {
    let mut start = 0;
    (text.split('\n').zip(source::style_lines(text)))
        .map(|(line, styled)| {
            let level = usize::from(styled.heading);
            let scale = (level.checked_sub(1))
                .and_then(|at| HEADING_SCALE.get(at))
                .unwrap_or(&1.0);
            let range = start..start + line.len();
            start = range.end + 1;
            SourceRow {
                range,
                spec: TextSpec {
                    size: body.size * scale,
                    line_height: body.line_height * scale,
                    ..*body
                },
                spans: styled.spans,
            }
        })
        .collect()
}

/// What sets one row apart from another with the same text.
type RowKey = (String, [u32; 3]);

fn row_key(text: &str, spec: &TextSpec) -> RowKey {
    let wrap = spec.wrap_width.map_or(u32::MAX, f32::to_bits);
    let bits = [spec.size.to_bits(), spec.line_height.to_bits(), wrap];
    (text.to_owned(), bits)
}

#[derive(Debug, Default)]
struct Kept {
    /// The latest whole layout, with the text and body spec it is of.
    whole: Option<(String, TextSpec, Arc<TextLayout>)>,
    /// The rows of that layout, so a keystroke measures only the row it
    /// changed.
    rows: HashMap<RowKey, Arc<TextLayout>>,
}

/// The layouts of the Document being edited, kept between the many times a
/// frame asks for them. It changes no answer, so a clone starts empty.
#[derive(Debug, Default)]
pub(crate) struct StackCache(Mutex<Kept>);

impl Clone for StackCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

/// `text` as stacked source rows, in one layout: offsets are into the whole
/// text and tops are measured from the first row.
pub(crate) fn layout(
    text: &str,
    body: &TextSpec,
    measure: &dyn TextMeasure,
    cache: &StackCache,
) -> Arc<TextLayout> {
    let mut kept = cache.0.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((kept_text, kept_body, whole)) = &kept.whole
        && kept_body == body
        && kept_text == text
    {
        return Arc::clone(whole);
    }
    let mut rows = HashMap::new();
    let mut lines = Vec::new();
    let mut top = 0.0;
    for row in source_rows(text, body) {
        let line = &text[row.range.clone()];
        let key = row_key(line, &row.spec);
        let measured = match kept.rows.remove(&key) {
            Some(measured) => measured,
            None => Arc::new(measure.layout_styled(line, &row.spec, &row.spans)),
        };
        lines.extend(measured.lines.iter().map(|line| {
            LayoutLine {
                range: line.range.start + row.range.start..line.range.end + row.range.start,
                top: line.top + top,
                height: line.height,
                stops: (line.stops.iter())
                    .map(|stop| super::CaretStop {
                        offset: stop.offset + row.range.start,
                        x: stop.x,
                    })
                    .collect(),
            }
        }));
        top += row_height(&measured, &row.spec);
        rows.insert(key, measured);
    }
    let whole = Arc::new(TextLayout { lines });
    kept.rows = rows;
    kept.whole = Some((text.to_owned(), *body, Arc::clone(&whole)));
    whole
}

/// How tall a row is: its lines, and never less than one line. The renderer
/// stacks rows by the same rule.
fn row_height(measured: &TextLayout, spec: &TextSpec) -> f32 {
    measured.height().max(spec.line_height)
}

#[cfg(test)]
mod tests {
    use specular_doc::{TextAlign, TextFont};

    use super::*;
    use crate::edit::measure::Measurer;

    fn body() -> TextSpec {
        TextSpec {
            font: TextFont::Sans,
            size: 10.0,
            line_height: 20.0,
            wrap_width: Some(200.0),
            align: TextAlign::Left,
        }
    }

    #[test]
    fn rows_take_their_heading_size_and_stack_in_one_layout() {
        let text = "# Top\nbody\n\n## Sub";
        let rows = source_rows(text, &body());
        assert_eq!(
            rows.iter()
                .map(|row| (row.range.clone(), row.spec.size))
                .collect::<Vec<_>>(),
            [(0..5, 14.0), (6..10, 10.0), (11..11, 10.0), (12..18, 12.0)]
        );
        let measure = Measurer::default();
        let layout = layout(text, &body(), measure.0.as_ref(), &StackCache::default());
        let lines: Vec<_> = (layout.lines.iter())
            .map(|line| (line.range.clone(), line.top, line.height))
            .collect();
        assert_eq!(
            lines,
            [
                (0..5, 0.0, 28.0),
                (6..10, 28.0, 20.0),
                (11..11, 48.0, 20.0),
                (12..18, 68.0, 24.0)
            ]
        );
        assert_eq!(layout.line_of(11), 2);
        assert_eq!(
            layout.lines[3].stops.first().map(|stop| stop.offset),
            Some(12)
        );
    }

    #[test]
    fn the_same_text_and_spec_is_laid_out_once() {
        let cache = StackCache::default();
        let measure = Measurer::default();
        let first = layout("a\nb", &body(), measure.0.as_ref(), &cache);
        let again = layout("a\nb", &body(), measure.0.as_ref(), &cache);
        assert!(Arc::ptr_eq(&first, &again));
        let edited = layout("a\nbc", &body(), measure.0.as_ref(), &cache);
        assert_eq!(edited.lines.len(), 2);
        assert!(!Arc::ptr_eq(&first, &edited));
    }
}
