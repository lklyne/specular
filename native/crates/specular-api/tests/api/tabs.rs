//! `--tab` while the user drags, which the CLI's end-to-end runs in
//! `fixtures/scenarios/cli` cannot do.

use serde_json::{Value, json};
use specular_api::{Request, Response};
use specular_doc::Rect;
use specular_interact::Effect;
use specular_testkit::{TestApp, document, pages, sticky};

use crate::common::Scripted;

fn space() -> Scripted {
    let notes = document([sticky("n1", Rect::new(0.0, 0.0, 200.0, 200.0), "hello")]);
    let mut app = TestApp::with_space([("Home", document(pages(2))), ("Notes", notes)]);
    app.viewport((1000.0, 800.0));
    Scripted::new(app)
}

fn tabbed(request: Request, tab: &str) -> Request {
    Request {
        tab: Some(tab.to_owned()),
        ..request
    }
}

#[track_caller]
fn ok(response: Response) -> Value {
    assert_eq!(response.status, 200, "{}", response.body);
    response.body
}

fn node_ids(canvas: &Value) -> Vec<&str> {
    (canvas["nodes"].as_array().into_iter().flatten())
        .filter_map(|node| node["id"].as_str())
        .collect()
}

#[test]
fn a_background_write_goes_through_while_the_user_drags() {
    let mut session = space();
    session.app.press((200.0, 150.0)).drag_to((260.0, 150.0));
    let patch = json!({ "delete": ["n1"] });
    ok(session.call(&tabbed(Request::post("/canvas/apply", patch), "Notes")));
    session.app.release();
    let read = ok(session.call(&tabbed(Request::get("/canvas"), "Notes")));
    assert_eq!(node_ids(&read), [] as [&str; 0]);
}

#[test]
fn a_background_add_of_a_file_outside_the_space_copies_it_in() {
    let mut session = space();
    let shot = specular_interact::DroppedFile {
        path: "/Users/me/Desktop/shot.png".to_owned(),
        space_path: None,
        image_size: Some((320, 180)),
    };
    session.disk.files.insert(shot.path.clone(), shot);
    let patch = json!({ "entities": [{ "kind": "file", "file": "/Users/me/Desktop/shot.png" }] });
    let created = ok(session.call(&tabbed(Request::post("/canvas/apply", patch), "Notes")));
    let id = created["created"][0]
        .as_str()
        .unwrap_or_default()
        .to_owned();
    let copy = format!("assets/{id}.png");
    assert!(
        session.effects.iter().any(|effect| matches!(
            effect,
            Effect::CopyAsset { from, file } if from == "/Users/me/Desktop/shot.png" && *file == copy
        )),
        "{:?}",
        session.effects
    );
}
