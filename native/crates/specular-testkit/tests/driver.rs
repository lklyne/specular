//! The driver itself: what each scripted input sends and what the assert
//! helpers catch.

use specular_core::{InputEvent, KeyEventKind};
use specular_interact::{Effect, Focus};
use specular_testkit::{ALT, TestApp, assert_doc_snapshot, document, pages};

const NOTE: &str = r#"{
  "nodes": [
    { "id": "n1", "type": "text", "x": 10.004, "y": 20, "width": 200, "height": 100, "text": "hi", "color": "3" }
  ],
  "edges": [],
  "theirs": { "kept": true }
}"#;

/// The characters forwarded to a page as typed text.
fn typed(effects: &[Effect]) -> String {
    let characters = effects.iter().filter_map(|effect| match effect {
        Effect::ForwardInput {
            event: InputEvent::Key(key),
            ..
        } if key.kind == KeyEventKind::Char => key.character,
        _ => None,
    });
    characters.collect()
}

#[test]
fn a_canvas_file_snapshots_one_line_per_item_in_the_writers_order() {
    let app = TestApp::from_canvas(NOTE);
    assert_doc_snapshot!(app, @r#"
    nodes:
      {"id":"n1","type":"text","x":10,"y":20,"width":200,"height":100,"text":"hi","color":"3"}
    edges:
    specular: {"entityOrder":["n1"]}
    theirs: {"kept":true}
    "#);
}

#[test]
fn opening_effects_are_dropped_by_constructors_and_kept_by_open() {
    let built = TestApp::with_pages(3);
    let mut opened = TestApp::empty();
    opened.open(document(pages(3)));
    assert_eq!((built.effects().len(), opened.effects().len()), (0, 3));
}

#[test]
fn a_double_click_enters_the_page_and_typed_text_reaches_it() {
    let mut app = TestApp::with_pages(2);
    app.double_click((800.0, 200.0)).take_effects();
    let effects = app.type_text("Hi there").take_effects();
    assert_eq!(
        (
            app.selected(),
            app.session().focus.page().is_some(),
            typed(&effects)
        ),
        (Some("p2"), true, "Hi there".to_owned())
    );
}

#[test]
fn held_modifiers_apply_until_let_go() {
    let mut app = TestApp::with_pages(1);
    app.hold(ALT).drag((200.0, 150.0), (210.0, 150.0)).let_go();
    let moved = app.rect("p1").x;
    app.drag((210.0, 150.0), (300.0, 150.0));
    assert_eq!((moved, app.rect("p1").x), (110.0, 110.0));
}

#[test]
fn the_wheel_pans_the_canvas_where_no_page_is_under_the_pointer() {
    let mut app = TestApp::with_pages(1);
    app.pointer_move((50.0, 50.0)).wheel((0.0, 40.0));
    assert_ne!(app.session().camera.pan.y, 0.0);
    assert_eq!(app.session().focus, Focus::Canvas);
}

#[test]
fn the_undo_assertion_leaves_the_document_and_the_effects_as_they_were() {
    let mut app = TestApp::with_pages(1);
    app.select(&["p1"]).drag((500.0, 400.0), (600.0, 450.0));
    let (before, effects) = (app.doc_snapshot(), app.effects().to_vec());
    app.assert_undo_returns_to_start();
    assert_eq!((app.doc_snapshot(), app.effects()), (before, &effects[..]));
    assert_eq!(effects.len(), 1);
}

#[test]
#[should_panic(expected = "a gesture is in flight")]
fn the_undo_assertion_refuses_to_run_mid_drag() {
    let mut app = TestApp::with_pages(1);
    app.hold(ALT).press((200.0, 150.0)).drag_to((300.0, 150.0));
    app.assert_undo_returns_to_start();
}
