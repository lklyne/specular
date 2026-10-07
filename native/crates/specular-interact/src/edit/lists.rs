//! Bullet lists in a text or sticky: Enter continues the list, Backspace
//! after a marker removes it, Tab and Shift+Tab nest and un-nest.
//!
//! A bullet line is optional indent, one of `-`, `*` or `+`, and a space.

use std::ops::Range;

use super::buffer::TextEdit;
use super::history::Change;
use super::segment;

/// One level of nesting.
const INDENT: &str = "  ";

/// The bytes of indent and of the whole marker (indent, bullet and space)
/// at the start of `line`, if it is a bullet line.
fn bullet(line: &str) -> Option<(usize, usize)> {
    let indent = line.len() - line.trim_start().len();
    let mut rest = line[indent..].chars();
    let marked = matches!(rest.next(), Some('-' | '*' | '+')) && rest.next() == Some(' ');
    marked.then_some((indent, indent + 2))
}

/// Enter: a line break. On a bullet line the new line starts with the same
/// marker, and on an empty bullet the marker goes instead, ending the list.
pub(crate) fn newline(edit: &mut TextEdit) -> bool {
    let line = segment::paragraph_at(&edit.text, edit.caret);
    let Some((indent, marker)) = bullet(&edit.text[line.clone()]) else {
        return edit.insert("\n", Change::Single);
    };
    if line.len() == marker {
        return edit.delete(line, Change::Single);
    }
    let continued = format!("\n{}- ", &edit.text[line.start..line.start + indent]);
    edit.insert(&continued, Change::Single)
}

/// Backspace with the caret just after a bullet's marker takes the marker
/// off and keeps the line. Returns whether it did.
pub(crate) fn unbullet(edit: &mut TextEdit) -> bool {
    if !edit.selection().is_empty() {
        return false;
    }
    let line = segment::paragraph_at(&edit.text, edit.caret);
    match bullet(&edit.text[line.clone()]) {
        Some((_, marker)) if edit.caret == line.start + marker => {
            edit.delete(line.start..line.start + marker, Change::Single)
        }
        _ => false,
    }
}

/// The bullet lines the selection touches, last first, so each can be
/// edited without moving the ones still to come.
fn selected_bullets(edit: &TextEdit) -> Vec<Range<usize>> {
    let selection = edit.selection();
    let mut lines = Vec::new();
    let mut at = segment::paragraph_at(&edit.text, selection.start).start;
    loop {
        let line = segment::paragraph_at(&edit.text, at);
        if bullet(&edit.text[line.clone()]).is_some() {
            lines.push(line.clone());
        }
        if line.end >= selection.end || line.end >= edit.text.len() {
            break;
        }
        at = line.end + 1;
    }
    lines.reverse();
    lines
}

/// Tab: nests every bullet line in the selection one level deeper.
pub(crate) fn indent(edit: &mut TextEdit) -> bool {
    let lines = selected_bullets(edit);
    edit.change(Change::Single, |edit| {
        for line in lines {
            edit.splice_around(line.start..line.start, INDENT);
        }
    })
}

/// Shift+Tab: takes one level of nesting off every bullet line in the
/// selection that has one.
pub(crate) fn outdent(edit: &mut TextEdit) -> bool {
    let lines = selected_bullets(edit);
    edit.change(Change::Single, |edit| {
        for line in lines {
            let text = &edit.text[line.clone()];
            let spaces = text.len() - text.trim_start_matches(' ').len();
            edit.splice_around(line.start..line.start + spaces.min(INDENT.len()), "");
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bullet_line_is_indent_a_marker_and_a_space() {
        assert_eq!(bullet("- a"), Some((0, 2)));
        assert_eq!(bullet("    * "), Some((4, 6)));
        assert_eq!(bullet("+ "), Some((0, 2)));
        assert_eq!(
            [bullet("-a"), bullet("a - b"), bullet(""), bullet("-")],
            [None; 4]
        );
    }
}
