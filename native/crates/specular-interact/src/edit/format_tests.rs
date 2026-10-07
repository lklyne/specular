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

fn wrapped(marked: &str, wrap: Wrap) -> String {
    after(marked, |e| toggle_wrap(e, wrap))
}

#[test]
fn wrap_puts_the_markers_around_the_selection_and_keeps_it_on_the_text() {
    assert_eq!(wrapped("a |bold| b", Wrap::Bold), "a **|bold|** b");
    assert_eq!(wrapped("a |x| b", Wrap::Italic), "a *|x|* b");
    assert_eq!(wrapped("a |x| b", Wrap::Code), "a `|x|` b");
    assert_eq!(wrapped("a |x| b", Wrap::Strike), "a ~~|x|~~ b");
}

#[test]
fn wrap_on_a_caret_inserts_the_pair_and_sits_between() {
    assert_eq!(wrapped("a | b", Wrap::Bold), "a **|** b");
}

#[test]
fn wrap_strips_markers_inside_or_outside_the_selection() {
    assert_eq!(wrapped("a **|bold|** b", Wrap::Bold), "a |bold| b");
    assert_eq!(wrapped("a |**bold**| b", Wrap::Bold), "a |bold| b");
    assert_eq!(wrapped("**|**", Wrap::Bold), "|");
}

#[test]
fn wrap_keeps_a_list_item_marker_outside() {
    assert_eq!(wrapped("|- item|", Wrap::Bold), "- **|item|**");
    assert_eq!(wrapped("|  1. item|", Wrap::Bold), "  1. **|item|**");
    assert_eq!(wrapped("|- [ ] item|", Wrap::Italic), "- [ ] *|item|*");
    assert_eq!(wrapped("|> ## quote|", Wrap::Code), "> ## `|quote|`");
    assert_eq!(wrapped("a\n|- one|\nb", Wrap::Bold), "a\n- **|one|**\nb");
}

#[test]
fn wrap_leaves_trailing_whitespace_outside() {
    assert_eq!(wrapped("|bold  |", Wrap::Bold), "**|bold|**  ");
}

#[test]
fn italic_on_the_inside_of_bold_wraps_again() {
    assert_eq!(wrapped("**|bold|**", Wrap::Italic), "***|bold|***");
    assert_eq!(wrapped("***|bold|***", Wrap::Italic), "**|bold|**");
    assert_eq!(wrapped("*|x|*", Wrap::Italic), "|x|");
    assert_eq!(wrapped("|**bold**|", Wrap::Italic), "*|**bold**|*");
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
    assert_eq!(
        after("日|本|語", |e| toggle_wrap(e, Wrap::Code)),
        "日`|本|`語"
    );
}

fn listed(marked: &str, kind: ListKind) -> String {
    after(marked, |e| toggle_list(e, kind))
}

#[test]
fn a_list_kind_is_added_after_the_indent() {
    assert_eq!(listed("|a|", ListKind::Bullet), "- |a|");
    assert_eq!(listed("  |a|", ListKind::Task), "  - [ ] |a|");
    assert_eq!(listed("a|b|", ListKind::Numbered), "1. a|b|");
}

#[test]
fn a_list_kind_is_stripped_when_every_line_has_it() {
    assert_eq!(listed("|- a\n- b|", ListKind::Bullet), "|a\nb|");
    assert_eq!(listed("|1. a\n2) b|", ListKind::Numbered), "|a\nb|");
    assert_eq!(listed("|- [ ] a\n- [x] b|", ListKind::Task), "|a\nb|");
}

#[test]
fn a_list_kind_replaces_the_other_markers() {
    assert_eq!(listed("|- [x] a|", ListKind::Bullet), "|- a|");
    assert_eq!(listed("|- a|", ListKind::Task), "|- [ ] a|");
    assert_eq!(listed("|1. a|", ListKind::Bullet), "|- a|");
    assert_eq!(
        listed("|* a\nb\n- [ ] c|", ListKind::Bullet),
        "|- a\n- b\n- c|"
    );
    assert_eq!(
        listed("|- a\n- [ ] b|", ListKind::Task),
        "|- [ ] a\n- [ ] b|"
    );
}

#[test]
fn a_task_line_is_not_a_bullet() {
    assert_eq!(listed("|- [ ] a\n- b|", ListKind::Bullet), "|- a\n- b|");
}

#[test]
fn numbered_lines_count_from_one_and_blank_lines_are_skipped() {
    assert_eq!(
        listed("|a\n\nb\n- c|", ListKind::Numbered),
        "1. |a\n\n2. b\n3. c|"
    );
    assert_eq!(listed("|5. a\n7. b|", ListKind::Numbered), "|a\nb|");
    assert_eq!(listed("|a\n\n\n|", ListKind::Bullet), "- |a\n\n\n|");
    assert_eq!(listed("|\n|", ListKind::Bullet), "|\n|");
}

#[test]
fn a_selection_ending_at_a_line_start_leaves_that_line_alone() {
    assert_eq!(listed("|a\n|b", ListKind::Bullet), "- |a\n|b");
    assert_eq!(listed("a|\nb|", ListKind::Bullet), "- a|\n- b|");
}

#[test]
fn a_list_toggle_is_one_undo_step_and_handles_multibyte_text() {
    let mut edit = session("|日本\n語|");
    assert!(toggle_list(&mut edit, ListKind::Task));
    assert_eq!(render(&edit), "- [ ] |日本\n- [ ] 語|");
    assert!(edit.undo());
    assert_eq!(render(&edit), "|日本\n語|");
}

fn headed(marked: &str, level: u8) -> String {
    after(marked, |e| set_heading(e, level))
}

#[test]
fn a_heading_is_set_changed_toggled_off_or_made_body() {
    assert_eq!(headed("a|b|", 2), "## a|b|");
    assert_eq!(headed("|# a|", 3), "|### a|");
    assert_eq!(headed("|## a|", 2), "|a|");
    assert_eq!(headed("|## a|", 0), "|a|");
    assert_eq!(headed("|a|", 0), "|a|");
    assert_eq!(headed("|  # a|", 2), "|  ## a|");
    assert_eq!(headed("|#a|", 1), "# |#a|");
    assert_eq!(headed("|####### a|", 1), "# |####### a|");
}

#[test]
fn a_heading_covers_every_spanned_line_but_blank_ones() {
    assert_eq!(headed("|a\n\n# b|", 1), "# |a\n\n# b|");
    assert_eq!(headed("|# a\n\n# b|", 1), "|a\n\nb|");
    assert_eq!(headed("|## a\n# b|", 1), "|# a\n# b|");
    assert_eq!(headed("|日本\n語|", 2), "## |日本\n## 語|");
}
