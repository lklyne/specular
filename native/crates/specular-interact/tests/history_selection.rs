//! The selection travels with history: an undo puts back what was selected
//! before the step, and a redo what was selected after it.

use specular_doc::Rect;
use specular_interact::Key;
use specular_testkit::{CMD, TestApp, shape};

fn three_shapes() -> TestApp {
    TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(300.0, 100.0, 100.0, 100.0)),
        shape("c", Rect::new(100.0, 300.0, 100.0, 100.0)),
    ])
}

fn copies(app: &TestApp) -> Vec<String> {
    let mut ids: Vec<String> = app.selected_ids().into_iter().map(str::to_owned).collect();
    ids.sort();
    ids
}

#[test]
fn undoing_a_duplicate_selects_the_originals_and_redoing_selects_the_copies() {
    let mut app = three_shapes();
    app.select(&["a", "b"]).chord(CMD, Key::Char('d'));
    let made = copies(&app);
    assert_eq!(made.len(), 2);
    assert!(!made.contains(&"a".to_owned()));

    app.undo();
    assert_eq!(app.selected_ids(), ["a", "b"]);
    app.redo();
    assert_eq!(copies(&app), made);

    // The selection an undo puts back is live: an arrow key moves it.
    app.undo().key(Key::ArrowRight);
    assert_ne!(app.rect("a").x, 100.0);
}

#[test]
fn undoing_a_delete_selects_what_came_back_and_redoing_selects_nothing() {
    let mut app = three_shapes();
    app.select(&["b"]).key(Key::Backspace).select(&["a"]);
    app.undo();
    assert_eq!(app.selected_ids(), ["b"]);
    app.redo();
    assert_eq!(app.selected_ids(), [] as [&str; 0]);
}

#[test]
fn undoing_a_move_selects_what_moved_even_after_the_selection_went_elsewhere() {
    let mut app = three_shapes();
    app.drag((150.0, 150.0), (150.0, 550.0)).select(&["b"]);
    app.undo();
    assert_eq!(app.selected_ids(), ["a"]);
    app.redo();
    assert_eq!(app.selected_ids(), ["a"]);
}
