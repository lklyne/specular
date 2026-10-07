//! The text editor's keys: typing, deleting, moving, selecting, the
//! clipboard, bullet lists and its own undo. Starting and ending an edit is
//! in `text_session.rs`, the pointer in `text_pointer.rs` and the input
//! method in `text_ime.rs`.
//!
//! Text is measured 10 units a character and 20 a line. The sticky under
//! edit is 200 wide, so its text wraps at 18 characters.

use specular_doc::Rect;
use specular_interact::{Effect, Key, TextEdit};
use specular_testkit::{ALT, CMD, CMD_SHIFT, SHIFT, TestApp, labelled, sticky};

const NOTE: Rect = Rect::new(100.0, 100.0, 200.0, 200.0);

/// A sticky reading `content`, double-clicked, with the caret put at the
/// start of its text.
fn editing(content: &str) -> TestApp {
    let mut app = TestApp::with_entities([sticky("n", NOTE, content)]);
    app.double_click((150.0, 250.0)).chord(CMD, Key::ArrowUp);
    app
}

#[test]
fn typing_goes_in_at_the_caret_and_over_a_selection() {
    let mut app = editing("held");
    app.key(Key::ArrowRight)
        .key(Key::ArrowRight)
        .type_text("llo wor");
    assert_eq!(app.editing_text(), "hello world");
    assert_eq!(app.caret(), (9, 9));

    app.chord(CMD, Key::Char('a')).type_text("Hi");
    assert_eq!((app.editing_text(), app.caret()), ("Hi", (2, 2)));
    assert_eq!(
        app.entity("n").rect,
        NOTE,
        "the document's text waits for the end"
    );
}

#[test]
fn backspace_and_delete_take_a_whole_grapheme() {
    let mut app = editing("a👨‍👩‍👧e\u{301}b");
    app.chord(CMD, Key::ArrowDown)
        .key(Key::Backspace)
        .key(Key::Backspace);
    assert_eq!(app.editing_text(), "a👨‍👩‍👧");
    app.key(Key::Backspace);
    assert_eq!(app.editing_text(), "a");
    app.key(Key::Home).key(Key::Delete).key(Key::Delete);
    assert_eq!((app.editing_text(), app.caret()), ("", (0, 0)));
}

#[test]
fn option_deletes_a_word_and_command_deletes_to_the_end_of_the_line() {
    let mut app = editing("one two three");
    app.chord(ALT, Key::Delete);
    assert_eq!(app.editing_text(), " two three");
    app.key(Key::End).chord(ALT, Key::Backspace);
    assert_eq!(app.editing_text(), " two ");
    app.chord(CMD, Key::Backspace);
    assert_eq!(app.editing_text(), "");

    let mut app = editing("one\ntwo");
    app.key(Key::ArrowRight).chord(CMD, Key::Delete);
    assert_eq!(app.editing_text(), "o\ntwo");
    // With nothing left on its side of the caret, it takes the line break.
    app.chord(CMD, Key::Delete);
    assert_eq!(app.editing_text(), "otwo");
}

#[test]
fn the_arrows_move_by_grapheme_word_line_and_document() {
    let mut app = editing("one two\nthree four");
    let at = |app: &mut TestApp, modifiers, key| app.chord(modifiers, key).caret().0;
    assert_eq!(at(&mut app, ALT, Key::ArrowRight), 3);
    assert_eq!(at(&mut app, CMD, Key::ArrowRight), 7);
    app.key(Key::ArrowRight);
    assert_eq!(app.caret(), (8, 8), "over the line break");
    assert_eq!(at(&mut app, ALT, Key::ArrowRight), 13);
    assert_eq!(at(&mut app, CMD, Key::ArrowDown), 18);
    assert_eq!(at(&mut app, ALT, Key::ArrowLeft), 14);
    assert_eq!(at(&mut app, CMD, Key::ArrowLeft), 8);
    app.key(Key::End);
    assert_eq!(app.caret(), (18, 18));
    app.key(Key::Home).key(Key::ArrowLeft);
    assert_eq!(app.caret(), (7, 7));
    assert_eq!(at(&mut app, CMD, Key::Home), 0);
    assert_eq!(at(&mut app, CMD, Key::End), 18);
}

#[test]
fn shift_extends_the_selection_and_a_plain_arrow_collapses_it_to_its_edge() {
    let mut app = editing("one two three");
    app.chord(ALT, Key::ArrowRight).hold(SHIFT);
    app.chord(ALT, Key::ArrowRight).key(Key::ArrowRight);
    assert_eq!(app.caret(), (8, 3));
    app.chord(CMD, Key::ArrowLeft);
    assert_eq!(
        app.caret(),
        (0, 3),
        "the anchor stays as the caret crosses it"
    );
    app.chord(CMD, Key::ArrowDown);
    assert_eq!(app.caret(), (13, 3));
    assert_eq!(app.app().text_edit().map(TextEdit::selection), Some(3..13));

    app.let_go().key(Key::ArrowLeft);
    assert_eq!(app.caret(), (3, 3), "left goes to the start and no further");
    app.hold(SHIFT)
        .chord(ALT, Key::ArrowRight)
        .let_go()
        .key(Key::ArrowRight);
    assert_eq!(app.caret(), (7, 7));
    app.chord(SHIFT, Key::ArrowLeft).type_text("x");
    assert_eq!(app.editing_text(), "one twx three");
}

#[test]
fn up_and_down_keep_their_column_across_a_short_line() {
    let mut app = editing("abcdef\nab\nabcdef");
    for _ in 0..5 {
        app.key(Key::ArrowRight);
    }
    assert_eq!(
        app.key(Key::ArrowDown).caret(),
        (9, 9),
        "the short line's end"
    );
    assert_eq!(
        app.key(Key::ArrowDown).caret(),
        (15, 15),
        "back out at column 5"
    );
    assert_eq!(app.key(Key::ArrowUp).key(Key::ArrowUp).caret(), (5, 5));
    assert_eq!(
        app.key(Key::ArrowUp).caret(),
        (0, 0),
        "up from the first line"
    );
    app.key(Key::ArrowDown).key(Key::ArrowDown);
    assert_eq!(
        app.caret(),
        (15, 15),
        "the column outlives the top of the text"
    );
    app.chord(CMD, Key::ArrowUp)
        .key(Key::ArrowDown)
        .key(Key::ArrowDown);
    assert_eq!(app.caret(), (10, 10), "any other move forgets it");
    assert_eq!(
        app.key(Key::ArrowDown).caret(),
        (16, 16),
        "down from the last line"
    );

    app.chord(CMD, Key::ArrowUp).key(Key::ArrowRight);
    app.hold(SHIFT).key(Key::ArrowDown).key(Key::ArrowDown);
    assert_eq!(app.caret(), (11, 1), "shift extends by lines");
}

#[test]
fn line_keys_follow_the_wrapped_lines() {
    // 18 characters fit, so the lines are "aaaa bbbb cccc " and "dddd eeee".
    let mut app = editing("aaaa bbbb cccc dddd eeee");
    assert_eq!(app.chord(CMD, Key::ArrowRight).caret(), (14, 14));
    assert_eq!(app.key(Key::ArrowDown).caret(), (24, 24));
    assert_eq!(app.chord(CMD, Key::ArrowLeft).caret(), (15, 15));
    assert_eq!(app.key(Key::ArrowUp).caret(), (0, 0));
    assert_eq!(app.key(Key::ArrowDown).key(Key::End).caret(), (24, 24));
    assert_eq!(app.key(Key::ArrowUp).caret(), (9, 9));
    app.chord(CMD, Key::Delete);
    assert_eq!(
        app.editing_text(),
        "aaaa bbbb dddd eeee",
        "to the end of the wrapped line"
    );
}

#[test]
fn select_all_copy_cut_and_paste_go_through_the_clipboard_effects() {
    let mut app = editing("hello world");
    app.chord(CMD, Key::Char('c'));
    app.take_effects();
    app.chord(CMD, Key::Char('a'));
    assert_eq!(app.caret(), (11, 0));
    app.chord(CMD, Key::Char('c'));
    assert_eq!(
        app.take_effects(),
        [Effect::WriteClipboard("hello world".to_owned())]
    );

    app.key(Key::ArrowLeft)
        .hold(SHIFT)
        .chord(ALT, Key::ArrowRight)
        .let_go();
    app.chord(CMD, Key::Char('x'));
    assert_eq!(
        app.take_effects(),
        [Effect::WriteClipboard("hello".to_owned())]
    );
    assert_eq!(app.editing_text(), " world");

    app.chord(CMD, Key::Char('v'));
    assert_eq!(app.take_effects(), [Effect::ReadClipboard]);
    assert_eq!(
        app.editing_text(),
        " world",
        "the paste waits for the answer"
    );
    app.paste("one\r\ntwo");
    assert_eq!(
        (app.editing_text(), app.caret()),
        ("one\ntwo world", (7, 7))
    );
}

#[test]
fn enter_continues_a_bullet_list_and_leaves_it_on_an_empty_item() {
    let mut app = editing("- one");
    app.key(Key::End).type_text("\ntwo");
    assert_eq!(app.editing_text(), "- one\n- two");
    app.key(Key::Enter);
    assert_eq!(app.editing_text(), "- one\n- two\n- ");
    app.key(Key::Enter);
    assert_eq!(
        (app.editing_text(), app.caret()),
        ("- one\n- two\n", (12, 12))
    );
    app.type_text("plain").chord(SHIFT, Key::Enter);
    assert_eq!(app.editing_text(), "- one\n- two\nplain\n");
}

#[test]
fn tab_nests_the_bullet_lines_in_the_selection_and_backspace_takes_a_marker_off() {
    let mut app = editing("- one\ntext\n  - two");
    app.key(Key::Tab);
    assert_eq!(
        (app.editing_text(), app.caret()),
        ("  - one\ntext\n  - two", (2, 2))
    );
    app.chord(CMD, Key::Char('a')).key(Key::Tab);
    assert_eq!(app.editing_text(), "    - one\ntext\n    - two");
    assert_eq!(app.caret(), (24, 2), "the selection moves with its text");
    app.chord(SHIFT, Key::Tab)
        .chord(SHIFT, Key::Tab)
        .chord(SHIFT, Key::Tab);
    assert_eq!(app.editing_text(), "- one\ntext\n- two");

    app.chord(CMD, Key::ArrowUp)
        .key(Key::ArrowDown)
        .key(Key::Tab);
    assert_eq!(
        app.editing_text(),
        "- one\ntext\n- two",
        "tab does nothing off a bullet"
    );
    app.chord(CMD, Key::ArrowUp)
        .key(Key::ArrowRight)
        .key(Key::ArrowRight);
    app.key(Key::Backspace);
    assert_eq!(
        (app.editing_text(), app.caret()),
        ("one\ntext\n- two", (0, 0))
    );
}

#[test]
fn a_shape_label_has_no_lists() {
    let mut app = TestApp::with_entities([labelled("s", NOTE, "- a")]);
    app.double_click((200.0, 200.0)).key(Key::ArrowRight);
    app.key(Key::Enter).key(Key::Tab);
    assert_eq!(app.editing_text(), "- a\n");
    app.key(Key::Backspace)
        .key(Key::ArrowLeft)
        .key(Key::Backspace);
    assert_eq!(app.editing_text(), "-a");
}

#[test]
fn undo_inside_an_edit_steps_through_runs_of_typing() {
    let mut app = editing("ab");
    app.key(Key::End)
        .type_text("cd")
        .key(Key::ArrowLeft)
        .type_text("x");
    app.key(Key::Backspace).key(Key::Backspace);
    assert_eq!(app.editing_text(), "abd");

    let undo = |app: &mut TestApp| app.chord(CMD, Key::Char('z')).editing_text().to_owned();
    assert_eq!(undo(&mut app), "abcxd", "the two backspaces are one step");
    assert_eq!(app.caret(), (4, 4));
    assert_eq!(undo(&mut app), "abcd");
    assert_eq!(undo(&mut app), "ab", "and so were the two letters");
    assert_eq!(undo(&mut app), "ab", "there is nothing more to undo");
    assert!(!app.app().can_undo(), "the document's history is untouched");

    app.chord(CMD_SHIFT, Key::Char('z'))
        .chord(CMD_SHIFT, Key::Char('z'));
    assert_eq!(app.editing_text(), "abcxd");
    app.type_text("!").chord(CMD_SHIFT, Key::Char('z'));
    assert_eq!(
        app.editing_text(),
        "abcx!d",
        "typing drops what could be redone"
    );
}

#[test]
fn keys_with_command_held_type_nothing_and_tool_keys_are_just_letters() {
    let mut app = editing("");
    app.type_text("vprc")
        .chord(CMD, Key::Char('d'))
        .chord(CMD, Key::Char('g'));
    assert_eq!(app.editing_text(), "vprc");
    assert_eq!(app.document().entities().count(), 1);
}
