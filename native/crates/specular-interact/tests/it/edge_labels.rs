//! Editing an edge's label in place. A double click on the edge starts it;
//! Enter, Escape and a press elsewhere keep what was typed, and a label
//! emptied is removed. The edge here runs a (100,100)-(300,200) to b
//! (500,100)-(700,200), so its middle is (400,150).

use specular_doc::{Edge, EdgeId, EdgeSide, Rect};
use specular_interact::Key;
use specular_testkit::{TestApp, document, shape, with_edge};

const MIDDLE: (f32, f32) = (400.0, 150.0);

fn linked(label: Option<&str>) -> TestApp {
    let edge = Edge {
        from_side: Some(EdgeSide::Right),
        to_side: Some(EdgeSide::Left),
        label: label.map(str::to_owned),
        ..Edge::new("e", "a", "b")
    };
    let start = document([
        shape("a", Rect::new(100.0, 100.0, 200.0, 100.0)),
        shape("b", Rect::new(500.0, 100.0, 200.0, 100.0)),
    ]);
    TestApp::from_document(with_edge(start, edge))
}

fn label(app: &TestApp) -> Option<String> {
    let edge = app.document().edge(&EdgeId::from("e"));
    edge.and_then(|edge| edge.label.clone())
}

#[test]
fn one_click_selects_the_edge_and_a_double_click_edits_its_label() {
    let mut app = linked(None);
    app.click(MIDDLE);
    assert!(app.app().text_edit().is_none());
    assert_eq!(app.selection().items().len(), 1);
    app.double_click(MIDDLE);
    assert!(app.app().text_edit().is_some());
    assert_eq!(app.editing_text(), "");
}

#[test]
fn typing_then_enter_sets_the_label_as_one_step() {
    let mut app = linked(None);
    app.double_click(MIDDLE).type_text("uses").key(Key::Enter);
    assert!(app.app().text_edit().is_none());
    assert_eq!(label(&app).as_deref(), Some("uses"));
    assert_eq!(app.selection().items().len(), 1, "the edge stays selected");
    app.undo();
    assert_eq!(label(&app), None);
    assert!(!app.app().can_undo());
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn an_emptied_label_removes_the_label() {
    let mut app = linked(Some("old"));
    app.double_click(MIDDLE).key(Key::Backspace).key(Key::Enter);
    assert_eq!(label(&app), None);
    app.undo();
    assert_eq!(label(&app).as_deref(), Some("old"));
    app.redo().assert_undo_returns_to_start();
}

#[test]
fn escape_and_a_press_elsewhere_keep_what_was_typed() {
    for (name, end) in [("escape", None), ("press elsewhere", Some((400.0, 600.0)))] {
        let mut app = linked(None);
        app.double_click(MIDDLE).type_text("kept");
        match end {
            None => app.key(Key::Escape),
            Some(at) => app.click(at),
        };
        assert!(app.app().text_edit().is_none(), "{name}");
        assert_eq!(label(&app).as_deref(), Some("kept"), "{name}");
        app.assert_undo_returns_to_start();
    }
}

#[test]
fn a_press_on_the_label_places_the_caret_and_keeps_editing() {
    let mut app = linked(None);
    app.double_click(MIDDLE).type_text("abcd");
    // The text spans 380 to 420 (10 units a character, centred on 400).
    app.click((390.0, 150.0));
    assert!(app.app().text_edit().is_some());
    assert_eq!(app.caret(), (1, 1));
}

#[test]
fn pasted_line_breaks_become_spaces() {
    let mut app = linked(None);
    app.double_click(MIDDLE).paste("a\nb").key(Key::Enter);
    assert_eq!(label(&app).as_deref(), Some("a b"));
}
