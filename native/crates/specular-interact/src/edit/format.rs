//! Markdown formatting as text transforms on the selection: wrap in inline
//! markers, turn lines into list items, set a heading level. Each is one
//! step of the editor's undo.

use std::ops::Range;

use super::buffer::TextEdit;
use super::history::Change;
use super::lists::{Mark, list_line};
use super::segment;

/// An inline marker pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wrap {
    /// `**`
    Bold,
    /// `*`
    Italic,
    /// `` ` ``
    Code,
    /// `~~`
    Strike,
}

impl Wrap {
    fn marker(self) -> &'static str {
        match self {
            Self::Bold => "**",
            Self::Italic => "*",
            Self::Code => "`",
            Self::Strike => "~~",
        }
    }
}

/// The kind of list item a line becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListKind {
    /// `- `
    Bullet,
    /// `1. `
    Numbered,
    /// `- [ ] `
    Task,
}

/// Bytes at the head of `line` that are block markup (indent, quote marks, a
/// heading mark, a list mark, a task box) rather than prose.
fn block_markup_end(line: &str) -> usize {
    let skip_spaces = |at: usize| line.len() - line[at..].trim_start_matches(' ').len();
    let mut at = skip_spaces(0);
    loop {
        let rest = &line[at..];
        let hashes = rest.bytes().take_while(|&b| b == b'#').count();
        let next = if rest.starts_with('>') {
            at + 1
        } else if (1..=6).contains(&hashes) && rest[hashes..].starts_with(' ') {
            at + hashes + 1
        } else if let Some((_, end, _)) = list_line(rest) {
            at + end
        } else {
            return at;
        };
        at = skip_spaces(next);
    }
}

/// `pos` moved past the block markup on its line.
fn inline_start(text: &str, pos: usize) -> usize {
    let line = segment::paragraph_at(text, pos);
    (line.start + block_markup_end(&text[line.clone()])).min(line.end)
}

/// The length of the run of `star`s at the end of `text`.
fn stars_before(text: &str) -> usize {
    text.bytes().rev().take_while(|&b| b == b'*').count()
}

/// The length of the run of `*`s at the start of `text`.
fn stars_after(text: &str) -> usize {
    text.bytes().take_while(|&b| b == b'*').count()
}

/// Whether single stars at both sides are an italic pair, rather than part
/// of a `**` pair. Three stars on each side is bold italic, whose italic is
/// the outermost star.
fn italic_pair(before: usize, after: usize) -> bool {
    (before == 1 && after == 1) || (before >= 3 && after >= 3)
}

/// Whether `head`, the prose of a line up to the caret, ends inside a run
/// that `wrap`'s marker opened and nothing has closed yet.
fn in_open_run(head: &str, wrap: Wrap) -> bool {
    let opened = match wrap {
        // A lone star, or the odd one of three, is italic. The rest are bold.
        Wrap::Italic => (head.split(|c| c != '*'))
            .filter(|stars| stars.len() % 2 == 1)
            .count(),
        Wrap::Bold | Wrap::Code | Wrap::Strike => head.matches(wrap.marker()).count(),
    };
    opened % 2 == 1
}

/// Wraps the selection in `wrap`'s markers, or takes them off if it is
/// already wrapped, whether they sit just outside the selection or are its
/// first and last characters. Block markup at the head of the selection stays
/// outside, so selecting a whole `- item` line wraps the item. An empty
/// selection gets the pair with the caret between, and a caret just before
/// the closing marker of a run steps past it: the shortcut, some words and
/// the shortcut again leave one pair, as in a rich-text editor.
pub(crate) fn toggle_wrap(edit: &mut TextEdit, wrap: Wrap) -> bool {
    let marker = wrap.marker();
    let width = marker.len();
    let selection = edit.selection();
    let start = selection
        .start
        .max(inline_start(&edit.text, selection.start));
    let end = selection.end.max(start);
    let end = start + edit.text[start..end].trim_end().len();
    let italic = wrap == Wrap::Italic;
    let (head, tail) = (&edit.text[..start], &edit.text[end..]);
    let outside = head.ends_with(marker)
        && tail.starts_with(marker)
        && (!italic || italic_pair(stars_before(head), stars_after(tail)));
    let closes_run = start == end
        && !outside
        && tail.starts_with(marker)
        && (!italic || stars_after(tail) % 2 == 1)
        && in_open_run(&head[inline_start(&edit.text, start).min(start)..], wrap);
    if closes_run {
        edit.move_to(start + width, false);
        return true;
    }
    let inner = &edit.text[start..end];
    let inside = inner.len() >= width * 2
        && inner.starts_with(marker)
        && inner.ends_with(marker)
        && (!italic || italic_pair(stars_after(inner), stars_before(inner)));

    edit.change(Change::Single, |edit| {
        let selected = if outside {
            edit.splice_around(end..end + width, "");
            edit.splice_around(start - width..start, "");
            start - width..end - width
        } else if inside {
            edit.splice_around(end - width..end, "");
            edit.splice_around(start..start + width, "");
            start..end - width * 2
        } else {
            edit.splice_around(end..end, marker);
            edit.splice_around(start..start, marker);
            start + width..end + width
        };
        edit.anchor = selected.start;
        edit.caret = selected.end;
    })
}

/// The lines the selection spans, first to last. A selection that ends
/// exactly at the start of a line has not selected anything on it.
fn selected_lines(edit: &TextEdit) -> Vec<Range<usize>> {
    let selection = edit.selection();
    let ends_at_line_start = selection.end > selection.start
        && segment::paragraph_at(&edit.text, selection.end).start == selection.end;
    let last = if ends_at_line_start {
        selection.end - 1
    } else {
        selection.end
    };
    let mut lines = Vec::new();
    let mut at = segment::paragraph_at(&edit.text, selection.start).start;
    loop {
        let line = segment::paragraph_at(&edit.text, at);
        lines.push(line.clone());
        if line.end >= last || line.end >= edit.text.len() {
            return lines;
        }
        at = line.end + 1;
    }
}

/// The non-blank lines the selection spans.
fn selected_text_lines(edit: &TextEdit) -> Vec<Range<usize>> {
    let mut lines = selected_lines(edit);
    lines.retain(|line| !edit.text[line.clone()].trim().is_empty());
    lines
}

/// Applies `replacements` (ranges in `edit.text`, in order) last first, so
/// each keeps the offsets of those before it.
fn apply(edit: &mut TextEdit, replacements: Vec<(Range<usize>, String)>) -> bool {
    edit.change(Change::Single, |edit| {
        for (range, with) in replacements.into_iter().rev() {
            edit.splice_around(range, &with);
        }
    })
}

/// Whether a list line already is a `kind` item.
fn has(kind: ListKind, mark: Mark) -> bool {
    matches!(
        (kind, mark),
        (ListKind::Bullet, Mark::Bullet(_))
            | (ListKind::Task, Mark::Task(..))
            | (ListKind::Numbered, Mark::Number(..))
    )
}

/// Makes every non-blank line the selection spans a `kind` item, replacing
/// whatever list marker it had; when they all are one already, takes the
/// markers off instead. Numbered items count 1, 2, 3 from the first line.
pub(crate) fn toggle_list(edit: &mut TextEdit, kind: ListKind) -> bool {
    let lines = selected_text_lines(edit);
    let parsed: Vec<_> = lines
        .iter()
        .map(|line| list_line(&edit.text[line.clone()]))
        .collect();
    let all = !lines.is_empty()
        && parsed
            .iter()
            .all(|p| p.is_some_and(|(_, _, mark)| has(kind, mark)));
    let replacements = lines
        .iter()
        .zip(&parsed)
        .enumerate()
        .map(|(index, (line, parsed))| {
            let text = &edit.text[line.clone()];
            let indent = parsed.map_or(text.len() - text.trim_start().len(), |(indent, ..)| indent);
            let end = parsed.map_or(indent, |(_, end, _)| end);
            if all {
                return (line.start..line.start + end, String::new());
            }
            let marker = match kind {
                ListKind::Bullet => "- ".to_owned(),
                ListKind::Task => "- [ ] ".to_owned(),
                ListKind::Numbered => format!("{}. ", index + 1),
            };
            (line.start + indent..line.start + end, marker)
        })
        .collect();
    apply(edit, replacements)
}

/// Where the heading mark is in `line`: its byte range, whole with the space
/// after it, and its level.
fn heading(line: &str) -> Option<(Range<usize>, u8)> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    let hashes = line[indent..].bytes().take_while(|&b| b == b'#').count();
    let marked = (1..=6).contains(&hashes) && line[indent + hashes..].starts_with(' ');
    marked.then(|| (indent..indent + hashes + 1, hashes as u8))
}

/// Sets heading `level` (1 to 6) on every non-blank line the selection
/// spans, or makes them body text for 0. A level they all have already is
/// taken off.
pub(crate) fn set_heading(edit: &mut TextEdit, level: u8) -> bool {
    let level = level.min(6);
    let lines = selected_text_lines(edit);
    let found: Vec<_> = lines
        .iter()
        .map(|line| heading(&edit.text[line.clone()]))
        .collect();
    let all = !lines.is_empty()
        && found
            .iter()
            .all(|h| h.as_ref().is_some_and(|(_, l)| *l == level));
    let replacements = lines
        .iter()
        .zip(found)
        .map(|(line, found)| {
            let text = &edit.text[line.clone()];
            let indent = text.len() - text.trim_start_matches(' ').len();
            let range = found.map_or(indent..indent, |(range, _)| range);
            let mark = if all || level == 0 {
                String::new()
            } else {
                format!("{} ", "#".repeat(level.into()))
            };
            (line.start + range.start..line.start + range.end, mark)
        })
        .collect();
    apply(edit, replacements)
}

#[cfg(test)]
#[path = "format_tests.rs"]
mod tests;
