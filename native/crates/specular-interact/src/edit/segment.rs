//! Where graphemes, words and paragraphs begin and end. Offsets are bytes
//! into the text and always sit on a grapheme boundary.

use std::ops::Range;

use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

/// The nearest grapheme boundary at or before `offset`.
pub(crate) fn floor(text: &str, offset: usize) -> usize {
    let mut at = offset.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    let mut cursor = GraphemeCursor::new(at, text.len(), true);
    match cursor.is_boundary(text, 0) {
        Ok(false) => previous(text, at),
        Ok(true) | Err(_) => at,
    }
}

/// The grapheme boundary before `offset`, or the start of the text.
pub(crate) fn previous(text: &str, offset: usize) -> usize {
    let mut cursor = GraphemeCursor::new(offset, text.len(), true);
    cursor.prev_boundary(text, 0).ok().flatten().unwrap_or(0)
}

/// The grapheme boundary after `offset`, or the end of the text.
pub(crate) fn next(text: &str, offset: usize) -> usize {
    let mut cursor = GraphemeCursor::new(offset, text.len(), true);
    let found = cursor.next_boundary(text, 0).ok().flatten();
    found.unwrap_or(text.len())
}

/// Whether a segment between two word boundaries is a word, as opposed to
/// the spaces and punctuation between words.
fn is_word(segment: &str) -> bool {
    segment.chars().any(char::is_alphanumeric)
}

/// The start of the word `offset` is in or after: where Option+Left goes.
pub(crate) fn word_start_before(text: &str, offset: usize) -> usize {
    let mut words = text
        .split_word_bound_indices()
        .filter(|(start, segment)| *start < offset && is_word(segment));
    words.next_back().map_or(0, |(start, _)| start)
}

/// The end of the word `offset` is in or before: where Option+Right goes.
pub(crate) fn word_end_after(text: &str, offset: usize) -> usize {
    let mut ends = text
        .split_word_bound_indices()
        .filter(|(_, segment)| is_word(segment))
        .map(|(start, segment)| start + segment.len());
    ends.find(|end| *end > offset).unwrap_or(text.len())
}

/// What a double click at `offset` selects: the word there, or the run of
/// spaces or punctuation when there is no word. A click on the boundary
/// after a word takes the word.
pub(crate) fn word_at(text: &str, offset: usize) -> Range<usize> {
    let mut before = None;
    for (start, segment) in text.split_word_bound_indices() {
        let range = start..start + segment.len();
        if range.end == offset {
            before = Some((range.clone(), is_word(segment)));
        }
        if range.start <= offset && offset < range.end {
            return match before {
                Some((word, true)) if !is_word(segment) => word,
                _ => range,
            };
        }
    }
    before.map_or(offset..offset, |(range, _)| range)
}

/// The paragraph `offset` is in: from after the line break before it to the
/// line break after it, which is left out.
pub(crate) fn paragraph_at(text: &str, offset: usize) -> Range<usize> {
    let offset = offset.min(text.len());
    let start = text[..offset].rfind('\n').map_or(0, |at| at + 1);
    let end = text[offset..]
        .find('\n')
        .map_or(text.len(), |at| offset + at);
    start..end
}

/// The byte offset `units` UTF-16 code units into `text`, held to its end.
pub(crate) fn from_utf16(text: &str, units: u32) -> usize {
    let mut seen = 0;
    for (at, character) in text.char_indices() {
        if seen >= units {
            return at;
        }
        seen += character.len_utf16() as u32;
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAMILY: &str = "a👨‍👩‍👧e\u{301}";

    #[test]
    fn a_grapheme_step_takes_a_whole_emoji_sequence_or_accent() {
        assert_eq!(next(FAMILY, 0), 1);
        assert_eq!(next(FAMILY, 1), 19);
        assert_eq!(next(FAMILY, 19), FAMILY.len());
        assert_eq!(previous(FAMILY, FAMILY.len()), 19);
        assert_eq!(previous(FAMILY, 19), 1);
        assert_eq!((previous(FAMILY, 0), next(FAMILY, 22)), (0, 22));
    }

    #[test]
    fn floor_pulls_an_offset_back_onto_a_boundary() {
        assert_eq!(
            [0, 1, 2, 7, 19, 20].map(|at| floor(FAMILY, at)),
            [0, 1, 1, 1, 19, 19]
        );
        assert_eq!(floor(FAMILY, 900), FAMILY.len());
    }

    #[test]
    fn word_steps_skip_spaces_and_punctuation() {
        let text = "one, two  three";
        assert_eq!(
            [15, 10, 9, 5, 4, 1].map(|at| word_start_before(text, at)),
            [10, 5, 5, 0, 0, 0]
        );
        assert_eq!(
            [0, 3, 4, 8, 12].map(|at| word_end_after(text, at)),
            [3, 8, 8, 15, 15]
        );
        assert_eq!(word_end_after(text, 15), 15);
    }

    #[test]
    fn a_double_click_takes_the_word_under_it_or_the_gap() {
        let text = "one, two  three";
        assert_eq!(word_at(text, 1), 0..3);
        assert_eq!(word_at(text, 3), 0..3, "the boundary after a word");
        assert_eq!(word_at(text, 8), 5..8);
        assert_eq!(word_at(text, 9), 8..10, "inside the gap");
        assert_eq!(word_at(text, 15), 10..15);
        assert_eq!(word_at("", 0), 0..0);
    }

    #[test]
    fn a_paragraph_stops_at_the_line_breaks_around_it() {
        let text = "one\ntwo\n";
        assert_eq!(
            [0, 3, 4, 7, 8].map(|at| paragraph_at(text, at)),
            [0..3, 0..3, 4..7, 4..7, 8..8]
        );
    }

    #[test]
    fn utf16_units_count_a_surrogate_pair_as_two() {
        assert_eq!(
            [0, 1, 3, 4, 99].map(|units| from_utf16("a😀b", units)),
            [0, 1, 5, 6, 6]
        );
    }
}
