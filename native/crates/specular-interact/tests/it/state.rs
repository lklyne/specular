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
    let mut every: Vec<String> = STATE_KEYS.iter().map(|key| (*key).to_owned()).collect();
    every.sort_unstable();
    assert_eq!(first, every);
    assert_eq!(
        publish_state(&mut revisions, app.app()),
        Vec::<String>::new()
    );
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
    assert_eq!(moved, Vec::<String>::new(), "the camera moved a key");
    app.act(Action::SetTool(specular_interact::Tool::AddSticky));
    let moved = publish_state(&mut revisions, app.app());
    assert!(moved.iter().any(|key| key == "panels/toolbar"), "{moved:?}");
    assert!(!moved.iter().any(|key| key == "keys"), "{moved:?}");
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
        assert!(
            !moved.iter().any(|it| it == key),
            "a move moved {key}: {moved:?}"
        );
    }
    app.assert_undo_returns_to_start();
}

/// Runs a command by name, as a mod's `run` does.
fn run(app: &mut TestApp, name: &str, args: &serde_json::Value) {
    let ran = specular_interact::run_named(app.app(), name, args);
    assert!(
        matches!(ran, Ok(specular_interact::Run::Action(_))),
        "{name} ran as {ran:?}"
    );
    if let Ok(specular_interact::Run::Action(action)) = ran {
        app.act(action);
    }
}

#[test]
fn a_thread_a_mod_starts_has_a_key_of_its_own_and_the_panel_keeps_its_thread() {
    let mut app = app();
    run(&mut app, "chat.new-thread", &serde_json::json!({}));
    let open = state(app.app())["chat/threads"]["open"].clone();
    assert!(open.is_string(), "the panel's New opens its thread");

    let mut revisions = Revisions::default();
    publish_state(&mut revisions, app.app());
    let args = serde_json::json!({ "id": "block_1", "open": false });
    run(&mut app, "chat.new-thread", &args);
    run(
        &mut app,
        "chat.send",
        &serde_json::json!({ "text": "hello", "thread": "block_1" }),
    );
    let read = state(app.app());
    assert_eq!(
        read["chat/threads"]["open"], open,
        "the panel's thread stays"
    );
    let thread = &read["chat/thread/block_1"];
    assert_eq!(thread["messages"][0]["text"], "hello");
    assert_eq!(thread["open"], false);
    let moved = publish_state(&mut revisions, app.app());
    let other = format!("chat/thread/{}", open.as_str().unwrap_or_default());
    assert!(
        moved.iter().any(|key| key == "chat/thread/block_1"),
        "{moved:?}"
    );
    assert!(
        !moved.contains(&other),
        "another thread's key moved: {moved:?}"
    );

    let taken = serde_json::json!({ "id": "../x" });
    assert!(specular_interact::run_named(app.app(), "chat.new-thread", &taken).is_err());

    // A closed thread's key reads null: a block showing it can say so.
    run(
        &mut app,
        "chat.new-thread",
        &serde_json::json!({ "id": "block_2", "open": false }),
    );
    publish_state(&mut revisions, app.app());
    assert!(
        revisions
            .get("chat/thread/block_2")
            .is_some_and(serde_json::Value::is_object)
    );
    run(
        &mut app,
        "chat.close",
        &serde_json::json!({ "id": "block_2" }),
    );
    publish_state(&mut revisions, app.app());
    assert_eq!(
        revisions.get("chat/thread/block_2"),
        Some(&serde_json::Value::Null)
    );
}

#[test]
fn a_view_sends_words_and_images_to_a_thread_it_names_and_that_starts_it() {
    let mut app = app();
    let open = state(app.app())["chat/threads"]["open"].clone();
    // A one-pixel PNG's first bytes are enough: the image is stored, not read.
    let image =
        serde_json::json!({ "type": "image/png", "data": "data:image/png;base64,iVBORw0KGgo=" });
    run(
        &mut app,
        "chat.send",
        &serde_json::json!({ "text": "this", "images": [image], "thread": "block_1" }),
    );
    let read = state(app.app());
    let message = &read["chat/thread/block_1"]["messages"][0];
    assert_eq!(message["text"], "this");
    assert_eq!(
        message["images"].as_array().map(Vec::len),
        Some(1),
        "{message}"
    );

    assert_eq!(read["chat/thread/block_1"]["open"], false);
    assert_eq!(
        read["chat/threads"]["open"], open,
        "the panel's thread stays"
    );

    let wrong = serde_json::json!({ "text": "", "images": [{ "type": "image/bmp", "data": "" }] });
    assert!(specular_interact::run_named(app.app(), "chat.send", &wrong).is_err());
}
