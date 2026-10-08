//! `update` asks for a save after every document change, and only then. A
//! reload from disk is a `DocumentOpened`, which keeps the session.

use specular_interact::{Action, Effect, Event};
use specular_testkit::{ALT, TestApp, document, pages};

/// A point on `p1` and one a short drag away.
const ON_P1: (f32, f32) = (200.0, 150.0);
const NEARBY: (f32, f32) = (260.0, 130.0);

fn saves(effects: &[Effect]) -> usize {
    let is_save = |effect: &&Effect| matches!(effect, Effect::Save);
    effects.iter().filter(is_save).count()
}

#[test]
fn a_drag_asks_for_one_save_when_it_is_released() {
    let mut app = TestApp::with_pages(2);
    app.take_effects();
    app.hold(ALT).press(ON_P1).drag_to(NEARBY);
    assert_eq!(saves(&app.take_effects()), 0, "nothing is saved mid-drag");
    app.release().let_go();
    assert_eq!(saves(&app.take_effects()), 1);
    app.assert_undo_returns_to_start();
}

#[test]
fn undo_and_redo_each_ask_for_a_save() {
    let mut app = TestApp::with_pages(2);
    app.hold(ALT).drag(ON_P1, NEARBY).let_go();
    app.take_effects();
    app.undo();
    assert_eq!(saves(&app.take_effects()), 1, "undo");
    app.redo();
    assert_eq!(saves(&app.take_effects()), 1, "redo");
    app.assert_undo_returns_to_start();
}

#[test]
fn what_leaves_the_document_alone_asks_for_nothing() {
    let mut app = TestApp::with_pages(2);
    app.take_effects();
    app.click(ON_P1)
        .wheel((0.0, 40.0))
        .pinch(0.1)
        .act(Action::Select(Vec::new()))
        .send(Event::Tick { unix_ms: 1_000 });
    assert_eq!(saves(&app.take_effects()), 0);
}

#[test]
fn opening_a_document_asks_for_nothing() {
    let mut app = TestApp::with_pages(2);
    app.hold(ALT).drag(ON_P1, NEARBY).let_go();
    app.take_effects();
    app.open(document(pages(3)));
    assert_eq!(saves(&app.take_effects()), 0, "what was just read is saved");
}

#[test]
fn a_reload_keeps_the_camera_and_drops_the_selection_that_is_gone() {
    let mut app = TestApp::with_pages(3);
    app.zoom(0.5).select(&["p1", "p3"]);
    let camera = app.session().camera;
    // The file as another tool left it: `p3` deleted.
    app.open(document(pages(2)));
    assert_eq!(app.session().camera, camera);
    assert_eq!(app.selected_ids(), ["p1"]);
}
