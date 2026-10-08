use specular_doc::{EntityId, Rect};

use super::super::buffer::{Origin, Target};
use super::*;

/// A session on `marked` without its `|`s: one is a caret, two are the
/// ends of a selection.
fn session(marked: &str) -> TextEdit {
    let text = marked.replace('|', "");
    let bars: Vec<usize> = marked.match_indices('|').map(|(at, _)| at).collect();
    let origin = Origin {
        text: text.clone(),
        rect: Rect::new(0.0, 0.0, 0.0, 0.0),
        created: false,
    };
    let mut edit = TextEdit::new(EntityId::from("t"), Target::Text, &text, origin);
    edit.select(bars[0]..bars.last().map_or(0, |last| last - (bars.len() - 1)));
    edit
}

/// The text with `|`s where the selection is.
fn render(edit: &TextEdit) -> String {
    let mut marks = [edit.anchor, edit.caret];
    marks.sort_unstable();
    let mut out = edit.text.clone();
    if marks[0] != marks[1] {
        out.insert(marks[1], '|');
    }
    out.insert(marks[0], '|');
    out
}

fn after(marked: &str, run: impl FnOnce(&mut TextEdit) -> bool) -> String {
    let mut edit = session(marked);
    run(&mut edit);
    render(&edit)
}

fn check_wraps(rows: &[(Wrap, &str, &str)]) {
    for (wrap, marked, want) in rows {
        let got = after(marked, |e| toggle_wrap(e, *wrap));
        assert_eq!(got, *want, "{wrap:?} on {marked}");
    }
}

fn check_lists(rows: &[(ListKind, &str, &str)]) {
    for (kind, marked, want) in rows {
        let got = after(marked, |e| toggle_list(e, *kind));
        assert_eq!(got, *want, "{kind:?} on {marked}");
    }
}

#[test]
fn wrap_puts_the_markers_around_the_selection_and_keeps_it_on_the_text() {
    check_wraps(&[
        (Wrap::Bold, "a |bold| b", "a **|bold|** b"),
        (Wrap::Italic, "a |x| b", "a *|x|* b"),
        (Wrap::Code, "a |x| b", "a `|x|` b"),
        (Wrap::Strike, "a |x| b", "a ~~|x|~~ b"),
        (Wrap::Bold, "a | b", "a **|** b"),
        (Wrap::Bold, "|bold  |", "**|bold|**  "),
    ]);
}

#[test]
fn wrap_on_a_caret_before_a_runs_closing_marker_steps_out_of_the_run() {
    check_wraps(&[
        (Wrap::Bold, "a **bold|** b", "a **bold**| b"),
        (Wrap::Italic, "a *x|* b", "a *x*| b"),
        (Wrap::Code, "a `x|` b", "a `x`| b"),
        (Wrap::Strike, "- ~~x|~~", "- ~~x~~|"),
        // Markers that follow a finished run open the next one.
        (Wrap::Bold, "**a** b|**c**", "**a** b**|****c**"),
        // A star of a bold pair is not an italic marker.
        (Wrap::Italic, "**a|**", "**a*|***"),
    ]);
}

#[test]
fn wrap_strips_markers_inside_or_outside_the_selection() {
    check_wraps(&[
        (Wrap::Bold, "a **|bold|** b", "a |bold| b"),
        (Wrap::Bold, "a |**bold**| b", "a |bold| b"),
        (Wrap::Bold, "**|**", "|"),
        (Wrap::Italic, "**|bold|**", "***|bold|***"),
        (Wrap::Italic, "***|bold|***", "**|bold|**"),
        (Wrap::Italic, "*|x|*", "|x|"),
        (Wrap::Italic, "|**bold**|", "*|**bold**|*"),
    ]);
}

#[test]
fn wrap_keeps_a_list_item_marker_outside() {
    check_wraps(&[
        (Wrap::Bold, "|- item|", "- **|item|**"),
        (Wrap::Bold, "|  1. item|", "  1. **|item|**"),
        (Wrap::Italic, "|- [ ] item|", "- [ ] *|item|*"),
        (Wrap::Code, "|> ## quote|", "> ## `|quote|`"),
        (Wrap::Bold, "a\n|- one|\nb", "a\n- **|one|**\nb"),
    ]);
}

#[test]
fn wrap_is_one_undo_step_and_counts_multibyte_text() {
    let mut edit = session("|日本語|");
    assert!(toggle_wrap(&mut edit, Wrap::Bold));
    assert_eq!(render(&edit), "**|日本語|**");
    assert!(toggle_wrap(&mut edit, Wrap::Bold));
    assert_eq!(render(&edit), "|日本語|");
    assert!(edit.undo());
    assert_eq!(render(&edit), "**|日本語|**");
    assert!(edit.undo());
    assert_eq!(render(&edit), "|日本語|");
    assert!(!edit.undo(), "each toggle left exactly one step");
    check_wraps(&[(Wrap::Code, "日|本|語", "日`|本|`語")]);
}

#[test]
fn a_list_kind_is_added_after_the_indent_and_a_selection_of_one_kind_removes_it() {
    check_lists(&[
        (ListKind::Bullet, "|a|", "- |a|"),
        (ListKind::Task, "  |a|", "  - [ ] |a|"),
        (ListKind::Numbered, "a|b|", "1. a|b|"),
        (ListKind::Bullet, "|- a\n- b|", "|a\nb|"),
        (ListKind::Numbered, "|1. a\n2) b|", "|a\nb|"),
        (ListKind::Task, "|- [ ] a\n- [x] b|", "|a\nb|"),
        (ListKind::Bullet, "|- [x] a|", "|- a|"),
        (ListKind::Task, "|- a|", "|- [ ] a|"),
        (ListKind::Bullet, "|1. a|", "|- a|"),
        (ListKind::Bullet, "|* a\nb\n- [ ] c|", "|- a\n- b\n- c|"),
        (ListKind::Task, "|- a\n- [ ] b|", "|- [ ] a\n- [ ] b|"),
        (ListKind::Bullet, "|- [ ] a\n- b|", "|- a\n- b|"),
    ]);
}

#[test]
fn numbered_lines_count_from_one_and_blank_lines_are_skipped() {
    check_lists(&[
        (ListKind::Numbered, "|a\n\nb\n- c|", "1. |a\n\n2. b\n3. c|"),
        (ListKind::Numbered, "|5. a\n7. b|", "|a\nb|"),
        (ListKind::Bullet, "|a\n\n\n|", "- |a\n\n\n|"),
        (ListKind::Bullet, "|\n|", "|\n|"),
        (ListKind::Bullet, "|a\n|b", "- |a\n|b"),
        (ListKind::Bullet, "a|\nb|", "- a|\n- b|"),
    ]);
}

#[test]
fn a_list_toggle_is_one_undo_step_and_handles_multibyte_text() {
    let mut edit = session("|日本\n語|");
    assert!(toggle_list(&mut edit, ListKind::Task));
    assert_eq!(render(&edit), "- [ ] |日本\n- [ ] 語|");
    assert!(edit.undo());
    assert_eq!(render(&edit), "|日本\n語|");
}

#[test]
fn a_heading_is_set_changed_toggled_off_or_made_body() {
    let rows = [
        ("a|b|", 2, "## a|b|"),
        ("|# a|", 3, "|### a|"),
        ("|## a|", 2, "|a|"),
        ("|## a|", 0, "|a|"),
        ("|a|", 0, "|a|"),
        ("|  # a|", 2, "|  ## a|"),
        ("|#a|", 1, "# |#a|"),
        ("|####### a|", 1, "# |####### a|"),
        ("|a\n\n# b|", 1, "# |a\n\n# b|"),
        ("|# a\n\n# b|", 1, "|a\n\nb|"),
        ("|## a\n# b|", 1, "|# a\n# b|"),
        ("|日本\n語|", 2, "## |日本\n## 語|"),
    ];
    for (marked, level, want) in rows {
        let got = after(marked, |e| set_heading(e, level));
        assert_eq!(got, want, "level {level} on {marked}");
    }
}
