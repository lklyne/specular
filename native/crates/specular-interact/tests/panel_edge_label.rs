//! The edge popup's label field: typed in the popup, kept with Enter as one
//! undo step, emptied to take the label off, and put back by Escape.

use specular_doc::{Edge, Rect};
use specular_interact::Key;
use specular_testkit::{TestApp, connected, document, shape, with_edge};

fn app() -> TestApp {
    let doc = connected(
        document([
            shape("a", Rect::new(100.0, 300.0, 100.0, 80.0)),
            shape("b", Rect::new(500.0, 300.0, 100.0, 80.0)),
        ]),
        "e",
        "a",
        "b",
    );
    let mut app = TestApp::empty();
    app.with_panels();
    app.open(doc);
    app.select(&["e"]);
    app
}

fn label(app: &TestApp) -> Option<String> {
    app.document()
        .edge(&"e".into())
        .and_then(|edge| edge.label.clone())
}

#[test]
fn a_label_typed_in_the_popup_is_kept_with_enter() {
    let mut app = app();
    app.enter_in_field("edge.label", "depends on");
    assert_eq!(label(&app), Some("depends on".to_owned()));
    app.undo();
    assert_eq!(label(&app), None);
    app.redo();
    app.assert_undo_returns_to_start();
}

#[test]
fn an_emptied_label_takes_the_label_off() {
    let doc = with_edge(
        document([
            shape("a", Rect::new(100.0, 300.0, 100.0, 80.0)),
            shape("b", Rect::new(500.0, 300.0, 100.0, 80.0)),
        ]),
        Edge {
            label: Some("old".to_owned()),
            ..Edge::new("e", "a", "b")
        },
    );
    let mut app = TestApp::empty();
    app.with_panels();
    app.open(doc);
    app.select(&["e"]);
    app.click_control("edge.label")
        .chord(specular_testkit::CMD, Key::Char('a'))
        .key(Key::Backspace)
        .key(Key::Enter);
    assert_eq!(label(&app), None);
    app.assert_undo_returns_to_start();
}

#[test]
fn escape_puts_the_old_label_back() {
    let mut app = app();
    app.click_control("edge.label")
        .type_text("draft")
        .key(Key::Escape);
    assert_eq!(label(&app), None);
    assert!(app.field_edit().is_none());
}
