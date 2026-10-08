//! Defects found by using the features together (the end-to-end QA pass),
//! each kept as the test that failed before its fix.

use specular_doc::{EntityId, ItemId, Rect};
use specular_interact::Key;
use specular_testkit::{CMD, TestApp, shape};

const BOX: Rect = Rect::new(100.0, 100.0, 100.0, 100.0);

fn one_shape() -> TestApp {
    TestApp::with_entities([shape("s", BOX)])
}

#[test]
fn a_moved_entity_stays_under_the_pointer_when_the_canvas_scrolls_mid_drag() {
    let mut app = one_shape();
    app.press((150.0, 150.0)).drag_to((250.0, 150.0));
    assert_eq!(app.rect("s").x, 200.0);
    // The canvas scrolls 100 px left under a pointer that does not move.
    app.wheel((-100.0, 0.0));
    assert_eq!(app.rect("s").x, 300.0, "still under the pointer");
    app.release();
    assert_eq!(app.rect("s").x, 300.0);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_marquee_corner_stays_under_the_pointer_when_the_canvas_scrolls_mid_drag() {
    let mut app = one_shape();
    app.press((20.0, 20.0)).drag_to((60.0, 60.0));
    assert_eq!(app.app().marquee_items(), []);
    // The shape scrolls in under the corner the pointer holds.
    app.wheel((-80.0, -80.0));
    let taken = app.app().marquee_items();
    assert_eq!(taken, [ItemId::Entity(EntityId::from("s"))]);
    app.key(Key::Escape).release();
}

#[test]
fn the_hover_follows_the_document_when_undo_moves_an_entity_from_under_the_pointer() {
    let mut app = one_shape();
    app.drag((150.0, 150.0), (450.0, 150.0))
        .pointer_move((450.0, 150.0));
    assert_eq!(app.session().hover, Some(EntityId::from("s")));
    app.chord(CMD, Key::Char('z'));
    assert_eq!(app.rect("s").x, 100.0);
    assert_eq!(
        app.session().hover,
        None,
        "nothing is under the pointer now"
    );
    app.redo();
    assert_eq!(app.session().hover, Some(EntityId::from("s")));
}
