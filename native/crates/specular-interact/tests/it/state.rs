//! The keyed state (ADR 0055): a key's revision moves only when its value
//! does, and the camera moves no panel's.

use glam::Vec2;
use specular_core::Camera;
use specular_doc::Rect;
use specular_interact::{Action, Event, Revisions, STATE_KEYS, publish_state, state};
use specular_testkit::{TestApp, page, sticky};

fn app() -> TestApp {
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(100.0, 100.0, 400.0, 300.0)),
        sticky("s", Rect::new(700.0, 100.0, 200.0, 200.0), "note"),
    ]);
    app.send(Event::ShellChrome);
    app.viewport((1200.0, 800.0));
    app.select(&["s"]);
    app
}

#[test]
fn every_key_is_published_and_publishing_again_moves_none() {
    let app = app();
    let mut revisions = Revisions::default();
    let first = publish_state(&mut revisions, app.app());
    let mut every = STATE_KEYS.to_vec();
    every.sort_unstable();
    assert_eq!(first, every);
    assert_eq!(publish_state(&mut revisions, app.app()), Vec::<&str>::new());
    assert!(
        state(app.app())["panels/dock"].is_object(),
        "the sticky's dock"
    );
}

#[test]
fn a_pan_or_a_zoom_moves_no_panel() {
    let mut app = app();
    let mut revisions = Revisions::default();
    publish_state(&mut revisions, app.app());
    app.act(Action::SetCamera(Camera::new(
        Vec2::new(-340.0, 120.0),
        1.0,
    )));
    app.act(Action::SetCamera(Camera::new(
        Vec2::new(-340.0, 120.0),
        2.5,
    )));
    let moved = publish_state(&mut revisions, app.app());
    assert_eq!(moved, Vec::<&str>::new(), "the camera moved a key");
    app.act(Action::SetTool(specular_interact::Tool::AddSticky));
    let moved = publish_state(&mut revisions, app.app());
    assert!(moved.contains(&"panels/toolbar"), "{moved:?}");
    assert!(!moved.contains(&"keys"), "{moved:?}");
}

#[test]
fn the_canvas_keys_name_what_is_there_and_a_move_moves_none_of_them() {
    let mut app = app();
    let read = state(app.app());
    assert_eq!(read["selection"]["ids"], serde_json::json!(["s"]));
    assert!(
        read["canvas/items"][0]["url"].is_string(),
        "a page's address"
    );
    assert_eq!(read["canvas/items"][1]["text"], "note");
    assert_eq!(read["canvas/info"]["items"], 2);
    let mut revisions = Revisions::default();
    publish_state(&mut revisions, app.app());
    app.act(Action::Nudge { dx: 40.0, dy: 0.0 });
    let moved = publish_state(&mut revisions, app.app());
    for key in ["selection", "canvas/items", "canvas/info", "panels/sidebar"] {
        assert!(!moved.contains(&key), "a move moved {key}: {moved:?}");
    }
    app.assert_undo_returns_to_start();
}
