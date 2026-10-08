//! Lists in a text or sticky: Enter continues the list, Backspace after a
//! marker removes it, Tab and Shift+Tab nest and un-nest.
//!
//! A list line is optional indent, then a bullet (`-`, `*` or `+`), a task
//! (a bullet and `[ ]` or `[x]`) or a number (digits and `.` or `)`), each
//! followed by a space.

use std::ops::Range;

use super::buffer::TextEdit;
use super::history::Change;
use super::segment;

/// One level of nesting.
const INDENT: &str = "  ";

/// What a list line starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mark {
    /// `-`, `*` or `+`.
    Bullet(char),
    /// A bullet char and whether the box is checked.
    Task(char, bool),
    /// The number and its delimiter, `.` or `)`.
    Number(u64, char),
}

/// The bytes of indent and of the whole marker (indent, mark and the space
/// after it) at the start of `line`, and the mark, if it is a list line.
pub(crate) fn list_line(line: &str) -> Option<(usize, usize, Mark)> {
    let indent = line.len() - line.trim_start().len();
    let rest = &line[indent..];
    let mut chars = rest.chars();
    let first = chars.next()?;
    if matches!(first, '-' | '*' | '+') {
        if chars.next() != Some(' ') {
            return None;
        }
        let after = indent + 2;
        let checked = match line[after..].get(..4) {
            Some("[ ] ") => Some(false),
            Some("[x] " | "[X] ") => Some(true),
            _ => None,
        };
        return Some(match checked {
            Some(checked) => (indent, after + 4, Mark::Task(first, checked)),
            None => (indent, after, Mark::Bullet(first)),
        });
    }
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let delimiter = rest[digits..]
        .chars()
        .next()
        .filter(|c| matches!(c, '.' | ')'));
    let number = rest[..digits].parse().ok().filter(|_| digits <= 9);
    match (number, delimiter) {
        (Some(n), Some(delimiter)) if rest[digits + 1..].starts_with(' ') => {
            Some((indent, indent + digits + 2, Mark::Number(n, delimiter)))
        }
        _ => None,
    }
}

/// Enter: a line break. On a list line the new line starts with the next
/// marker, and on an empty item the marker goes instead, ending the list.
pub(crate) fn newline(edit: &mut TextEdit) -> bool {
    let line = segment::paragraph_at(&edit.text, edit.caret);
    let Some((indent, marker, mark)) = list_line(&edit.text[line.clone()]) else {
        return edit.insert("\n", Change::Single);
    };
    if line.len() == marker {
        return edit.delete(line, Change::Single);
    }
    let next = match mark {
        Mark::Bullet(bullet) => format!("{bullet} "),
        Mark::Task(bullet, _) => format!("{bullet} [ ] "),
        Mark::Number(n, delimiter) => format!("{}{delimiter} ", n + 1),
    };
    let continued = format!("\n{}{next}", &edit.text[line.start..line.start + indent]);
    edit.insert(&continued, Change::Single)
}

/// Backspace with the caret just after a list marker takes the marker off
/// and keeps the line. Returns whether it did.
pub(crate) fn unbullet(edit: &mut TextEdit) -> bool {
    if !edit.selection().is_empty() {
        return false;
    }
    let line = segment::paragraph_at(&edit.text, edit.caret);
    match list_line(&edit.text[line.clone()]) {
        Some((_, marker, _)) if edit.caret == line.start + marker => {
            edit.delete(line.start..line.start + marker, Change::Single)
        }
        _ => false,
    }
}

/// The list lines the selection touches, last first, so each can be
/// edited without moving the ones still to come.
fn selected_bullets(edit: &TextEdit) -> Vec<Range<usize>> {
    let selection = edit.selection();
    let mut lines = Vec::new();
    let mut at = segment::paragraph_at(&edit.text, selection.start).start;
    loop {
        let line = segment::paragraph_at(&edit.text, at);
        if list_line(&edit.text[line.clone()]).is_some() {
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

/// Tab: nests every list line in the selection one level deeper.
pub(crate) fn indent(edit: &mut TextEdit) -> bool {
    let lines = selected_bullets(edit);
    edit.change(Change::Single, |edit| {
        for line in lines {
            edit.splice_around(line.start..line.start, INDENT);
        }
    })
}

/// Shift+Tab: takes one level of nesting off every list line in the
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
    use specular_doc::{EntityId, Rect};

    use super::super::buffer::{Origin, Target};
    use super::*;

    #[test]
    fn a_list_line_is_indent_a_mark_and_a_space() {
        assert_eq!(list_line("- a"), Some((0, 2, Mark::Bullet('-'))));
        assert_eq!(list_line("    * "), Some((4, 6, Mark::Bullet('*'))));
        assert_eq!(list_line("+ "), Some((0, 2, Mark::Bullet('+'))));
        assert_eq!(list_line("- [ ] a"), Some((0, 6, Mark::Task('-', false))));
        assert_eq!(list_line(" * [X] "), Some((1, 7, Mark::Task('*', true))));
        assert_eq!(list_line("12) a"), Some((0, 4, Mark::Number(12, ')'))));
        assert_eq!(list_line("1. "), Some((0, 3, Mark::Number(1, '.'))));
        assert_eq!(list_line("- [] a"), Some((0, 2, Mark::Bullet('-'))));
        assert_eq!(
            [
                list_line("-a"),
                list_line("a - b"),
                list_line(""),
                list_line("-"),
                list_line("1.a"),
                list_line("1 a"),
                list_line(".")
            ],
            [None; 7]
        );
    }

    fn edit(text: &str, caret: usize) -> TextEdit {
        let origin = Origin {
            text: text.to_owned(),
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            created: false,
        };
        let mut edit = TextEdit::new(EntityId::from("t"), Target::Text, text, origin);
        edit.select(caret..caret);
        edit
    }

    fn enter(text: &str) -> String {
        let mut e = edit(text, text.len());
        newline(&mut e);
        e.text
    }

    #[test]
    fn enter_continues_each_kind_of_list_line() {
        assert_eq!(enter("* a"), "* a\n* ");
        assert_eq!(enter("  + a"), "  + a\n  + ");
        assert_eq!(enter("1. a"), "1. a\n2. ");
        assert_eq!(enter("9) a"), "9) a\n10) ");
        assert_eq!(enter("- [x] a"), "- [x] a\n- [ ] ");
        assert_eq!(enter("  * [ ] a"), "  * [ ] a\n  * [ ] ");

        {
            assert_eq!(enter("a\n2. "), "a\n");
            assert_eq!(enter("a\n- [ ] "), "a\n");
            assert_eq!(enter("a\n* "), "a\n");
        }
    }

    #[test]
    fn backspace_after_a_marker_removes_all_of_it() {
        for (text, at) in [("1. a", 3), ("- [ ] a", 6), ("  3) a", 5)] {
            let mut e = edit(text, at);
            assert!(unbullet(&mut e), "{text}");
            assert_eq!(e.text, "a");
            assert_eq!(e.caret, 0);
        }
        let mut e = edit("- [ ] a", 2);
        assert!(!unbullet(&mut e), "inside a task's box is plain text");
    }

    #[test]
    fn tab_and_shift_tab_nest_numbered_and_task_lines() {
        let mut e = edit("1. a\n- [ ] b", 0);
        e.select(0..e.text.len());
        assert!(indent(&mut e));
        assert_eq!(e.text, "  1. a\n  - [ ] b");
        assert!(outdent(&mut e));
        assert_eq!(e.text, "1. a\n- [ ] b");
    }
}
