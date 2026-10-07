//! Caret movement: where each motion takes the caret, and deleting up to
//! there.

use std::ops::Range;

use super::buffer::TextEdit;
use super::history::Change;
use super::measure::TextLayout;
use super::segment;

/// A way the caret moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Motion {
    /// One grapheme back.
    Left,
    /// One grapheme forward.
    Right,
    /// To the start of the word.
    WordLeft,
    /// To the end of the word.
    WordRight,
    /// To the start of the visual line.
    LineStart,
    /// To the end of the visual line.
    LineEnd,
    /// To the start of the text.
    DocStart,
    /// To the end of the text.
    DocEnd,
    /// One visual line up, keeping the column.
    Up,
    /// One visual line down, keeping the column.
    Down,
    /// To the start of the paragraph, or of the one before from there.
    ParagraphUp,
    /// To the end of the paragraph, or of the one after from there.
    ParagraphDown,
    /// Up by what the text's window shows, keeping the column.
    PageUp,
    /// Down by what the text's window shows, keeping the column.
    PageDown,
}

/// The text as the motions see it: its lines, and how much of them shows at
/// once, which is what Page Up and Page Down move by.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Seen<'a> {
    pub(crate) layout: &'a TextLayout,
    pub(crate) page: f32,
}

/// Where `motion` takes the caret, and the column it is aiming for if it
/// is a vertical one.
fn destination(
    edit: &TextEdit,
    motion: Motion,
    seen: Seen<'_>,
    from: usize,
) -> (usize, Option<f32>) {
    let layout = seen.layout;
    let text = edit.text.as_str();
    let vertical = |onto: Option<usize>, edge: usize| {
        let x = edit.preferred_x.unwrap_or_else(|| layout.x_of(from));
        (onto.map_or(edge, |line| layout.offset_at(line, x)), Some(x))
    };
    let line = layout.line_of(from);
    match motion {
        Motion::Left => (segment::previous(text, from), None),
        Motion::Right => (segment::next(text, from), None),
        Motion::WordLeft => (segment::word_start_before(text, from), None),
        Motion::WordRight => (segment::word_end_after(text, from), None),
        Motion::LineStart => (layout.line_start(from), None),
        Motion::LineEnd => (layout.line_end(from), None),
        Motion::DocStart => (0, None),
        Motion::DocEnd => (text.len(), None),
        // Up from the first line goes to the start of the text and down from
        // the last to its end, as a macOS text field does.
        Motion::Up => vertical(line.checked_sub(1), 0),
        Motion::Down => vertical(
            Some(line + 1).filter(|next| *next < layout.lines.len()),
            text.len(),
        ),
        Motion::ParagraphUp => {
            let start = segment::paragraph_at(text, from).start;
            let before = segment::paragraph_at(text, start.saturating_sub(1)).start;
            (if from > start { start } else { before }, None)
        }
        Motion::ParagraphDown => {
            let end = segment::paragraph_at(text, from).end;
            let after = segment::paragraph_at(text, (end + 1).min(text.len())).end;
            (if from < end { end } else { after }, None)
        }
        Motion::PageUp | Motion::PageDown => {
            let top = layout.lines.get(line).map_or(0.0, |line| line.top);
            let (aim, edge) = if motion == Motion::PageUp {
                (top - seen.page, 0)
            } else {
                (top + seen.page, text.len())
            };
            // A page that moves no line has reached the first or the last.
            vertical(Some(layout.line_at(aim)).filter(|onto| *onto != line), edge)
        }
    }
}

/// Moves the caret by `motion`. `extend` keeps the anchor. Without it a
/// selection collapses: left and up leave from its start, right and down
/// from its end, and a single step left or right goes no further than that
/// edge.
pub(crate) fn apply(edit: &mut TextEdit, motion: Motion, extend: bool, seen: Seen<'_>) {
    let selection = edit.selection();
    let collapses = !extend && !selection.is_empty();
    let from = match motion {
        _ if !collapses => edit.caret,
        Motion::Left
        | Motion::WordLeft
        | Motion::LineStart
        | Motion::DocStart
        | Motion::Up
        | Motion::ParagraphUp
        | Motion::PageUp => selection.start,
        Motion::Right
        | Motion::WordRight
        | Motion::LineEnd
        | Motion::DocEnd
        | Motion::Down
        | Motion::ParagraphDown
        | Motion::PageDown => selection.end,
    };
    let (to, column) = match motion {
        Motion::Left | Motion::Right if collapses => (from, None),
        Motion::Left
        | Motion::Right
        | Motion::WordLeft
        | Motion::WordRight
        | Motion::LineStart
        | Motion::LineEnd
        | Motion::DocStart
        | Motion::DocEnd
        | Motion::Up
        | Motion::Down
        | Motion::ParagraphUp
        | Motion::ParagraphDown
        | Motion::PageUp
        | Motion::PageDown => destination(edit, motion, seen, from),
    };
    edit.move_to(to, extend);
    edit.preferred_x = column;
}

/// What a delete key with `motion` removes: the selection, or from the
/// caret to where the motion goes. A line delete with nothing on its side
/// of the caret takes the line break instead.
fn doomed(edit: &TextEdit, motion: Motion, seen: Seen<'_>) -> Range<usize> {
    let selection = edit.selection();
    if !selection.is_empty() {
        return selection;
    }
    let caret = edit.caret;
    let span = |to: usize| caret.min(to)..caret.max(to);
    let range = span(destination(edit, motion, seen, caret).0);
    match motion {
        Motion::LineStart if range.is_empty() => span(segment::previous(&edit.text, caret)),
        Motion::LineEnd if range.is_empty() => span(segment::next(&edit.text, caret)),
        _ => range,
    }
}

/// Deletes the selection, or from the caret to where `motion` goes.
/// Returns whether the text changed.
pub(crate) fn delete(edit: &mut TextEdit, motion: Motion, seen: Seen<'_>) -> bool {
    let change = match motion {
        Motion::Left | Motion::Right if edit.selection().is_empty() => Change::Deleting,
        _ => Change::Single,
    };
    let range = doomed(edit, motion, seen);
    edit.delete(range, change)
}
