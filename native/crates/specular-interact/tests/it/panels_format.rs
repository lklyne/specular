//! The formatting buttons of the dock while a text is edited: each is on
//! where the caret is in text it applies to, and pressing one runs the format
//! on the selection without ending the edit or losing the selection.

use specular_doc::Rect;
use specular_interact::Key;
use specular_testkit::{CMD, SHIFT, TestApp, note, sticky};

const NOTE: Rect = Rect::new(100.0, 100.0, 300.0, 200.0);

/// A sticky reading `content`, double-clicked, with the caret at the start.
fn editing(content: &str) -> TestApp {
    let mut app = TestApp::with_entities([sticky("n", NOTE, content)]);
    app.with_panels();
    app.double_click((150.0, 200.0)).chord(CMD, Key::ArrowUp);
    app
}

fn on(app: &TestApp, name: &str) -> bool {
    let line = format!("toggle [x] format.{name} ");
    app.dock_snapshot().contains(&line)
}

fn move_right(app: &mut TestApp, count: usize) {
    for _ in 0..count {
        app.key(Key::ArrowRight);
    }
}

#[test]
fn bold_is_on_inside_a_bold_run_and_off_outside() {
    let mut app = editing("a **bold** b");
    assert!(!on(&app, "bold"));
    move_right(&mut app, 6);
    assert_eq!(app.caret(), (6, 6));
    assert!(on(&app, "bold"), "between the markers");
    move_right(&mut app, 6);
    assert!(!on(&app, "bold"), "after the closing markers");

    // A marker nothing closes opens no run.
    let mut app = editing("a **open");
    move_right(&mut app, 5);
    assert!(!on(&app, "bold"), "inside an unclosed run");

    let mut app = editing("a **bold** b");
    move_right(&mut app, 2);
    app.hold(SHIFT);
    move_right(&mut app, 8);
    app.let_go();
    assert!(on(&app, "bold"), "a selection that holds its markers");
    assert!(!on(&app, "strikethrough"));

    let mut app = editing("- one ~~two~~\nplain");
    move_right(&mut app, 10);
    assert!(on(&app, "strikethrough"));
    assert!(on(&app, "bullets"), "the line is a list item");
    app.key(Key::ArrowDown);
    assert!(!on(&app, "strikethrough"));
    assert!(!on(&app, "bullets"));
}

#[test]
fn pressing_a_button_formats_the_selection_and_keeps_the_edit_and_the_selection() {
    let mut app = editing("make this bold");
    move_right(&mut app, 5);
    app.hold(SHIFT);
    move_right(&mut app, 4);
    app.let_go();
    assert_eq!(app.caret(), (9, 5));

    app.click_control("format.bold");
    assert_eq!(app.editing_text(), "make **this** bold");
    assert!(app.app().session().editing.is_some(), "the edit goes on");
    assert_eq!(app.caret(), (11, 7), "this is still selected");
    assert!(on(&app, "bold"));

    app.click_control("format.bold");
    assert_eq!(app.editing_text(), "make this bold");
    assert!(!on(&app, "bold"));

    app.click_control("format.bullets");
    assert_eq!(app.editing_text(), "- make this bold");
    assert!(on(&app, "bullets"));
    app.key(Key::Escape);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_documents_buttons_wait_disabled_until_it_is_edited() {
    let mut app = TestApp::with_entities([note("d", NOTE, "plan.md")]);
    app.with_panels();
    app.note_text("plan.md", "one\ntwo");
    app.select(&["d"]);
    let idle = app.dock_snapshot();
    assert!(idle.contains("toggle [ ] format.bold \"Bold\" icon=Bold chord=cmd+b disabled"));
    app.click_control("format.bold");
    assert!(
        app.app().session().editing.is_none(),
        "a disabled press does nothing"
    );

    app.double_click((150.0, 150.0));
    assert!(app.app().session().editing.is_some());
    assert!(!app.dock_snapshot().contains("disabled"));
    app.chord(CMD, Key::Char('a'));
    app.click_control("format.bold");
    assert_eq!(app.editing_text(), "**one\ntwo**");
    assert!(app.app().session().editing.is_some());
    app.key(Key::Escape);
    app.assert_undo_returns_to_start();
}
