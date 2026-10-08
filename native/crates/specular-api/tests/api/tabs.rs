//! The tab verbs and `--tab`: listing the canvases of the space, making,
//! showing and deleting one, and reading and writing one in the background.

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
fn tabs_lists_every_canvas_and_the_active_one() {
    let mut session = space();
    assert_eq!(
        ok(session.get("/tabs")),
        json!({
            "activeTab": { "id": "tab_1", "name": "Home" },
            "tabs": [
                { "id": "tab_1", "name": "Home", "entityCount": 2 },
                { "id": "tab_2", "name": "Notes", "entityCount": 1 },
            ],
        })
    );
    session.app.switch_to("Notes");
    assert_eq!(
        ok(session.get("/tabs"))["activeTab"],
        json!({ "id": "tab_2", "name": "Notes" })
    );
}

#[test]
fn a_new_tab_is_made_in_the_background_and_a_taken_name_is_refused() {
    let mut session = space();
    let made = ok(session.post("/tabs", json!({ "name": " Research " })));
    assert_eq!(made["name"], "Research");
    assert_eq!(made["activated"], false);
    assert_eq!(session.app.active_canvas(), "Home");
    assert_eq!(session.app.canvas_names(), ["Home", "Notes", "Research"]);
    assert_eq!(made["id"], session.app.canvas_id("Research").as_str());
    let written = (session.effects.iter()).any(|effect| matches!(effect, Effect::WriteCanvas(_)));
    assert!(written, "the new canvas gets its file");

    let again = session.post("/tabs", json!({ "name": "Research" }));
    assert_eq!(
        (again.status, again.body),
        (
            400,
            json!({ "error": "a tab named 'Research' already exists" })
        )
    );
    let unnamed = session.post("/tabs", json!({}));
    assert_eq!(
        (unnamed.status, unnamed.body),
        (400, json!({ "error": "tab name is required" }))
    );
}

#[test]
fn switch_shows_the_canvas_a_ref_names() {
    let mut session = space();
    let switched = ok(session.post("/tabs/switch", json!({ "ref": "Notes" })));
    assert_eq!(
        switched,
        json!({ "activeTab": { "id": "tab_2", "name": "Notes" } })
    );
    assert_eq!(session.app.active_canvas(), "Notes");
    assert_eq!(node_ids(&session.canvas()), ["n1"]);

    let unknown = session.post("/tabs/switch", json!({ "ref": "nope" }));
    assert_eq!(
        (unknown.status, unknown.body),
        (
            400,
            json!({ "error": "unknown tab 'nope' \u{2014} available: tab_1 (Home), tab_2 (Notes)" })
        )
    );
}

#[test]
fn delete_removes_a_background_canvas_and_resets_the_last_one() {
    let mut session = space();
    let deleted = ok(session.post("/tabs/delete", json!({ "ref": "tab_2" })));
    assert_eq!(
        deleted,
        json!({
            "deleted": { "id": "tab_2", "name": "Notes" },
            "reset": false,
            "activeTab": { "id": "tab_1", "name": "Home" },
        })
    );
    let last = ok(session.post("/tabs/delete", json!({ "ref": "Home" })));
    assert_eq!(last["reset"], true);
    assert_eq!(last["activeTab"]["name"], "Canvas 1");
    assert_eq!(session.app.canvas_names(), ["Canvas 1"]);
}

#[test]
fn a_tab_ref_writes_a_background_canvas_without_showing_it() {
    let mut session = space();
    session.app.select(&["p1"]);
    let patch = json!({ "entities": [
        { "kind": "text", "text": "from an agent" },
        { "kind": "page", "url": "https://example.com/" },
    ] });
    let done = ok(session.call(&tabbed(Request::post("/canvas/apply", patch), "Notes")));
    let created: Vec<&str> = (done["created"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect();
    let [text, page] = created.as_slice() else {
        panic!("two entities, not {created:?}");
    };

    // The user's canvas, selection and undo stack are as they were.
    assert_eq!(session.app.active_canvas(), "Home");
    assert_eq!(session.app.selected(), Some("p1"));
    assert!(!session.app.app().can_undo());
    // The canvas is written, and no page is hosted for it yet.
    let notes = session.app.canvas_id("Notes");
    assert_eq!(session.effects, [Effect::WriteCanvas(notes)]);
    assert_eq!(node_ids(&session.canvas()), ["p1", "p2"]);

    let read = ok(session.call(&tabbed(Request::get("/canvas"), "Notes")));
    assert_eq!(node_ids(&read), ["n1", text, page]);

    // Its page is hosted when the canvas is shown, and the write is one
    // undo step there.
    session.app.take_effects();
    session.app.switch_to("Notes");
    let hosted = (session.app.effects().iter())
        .any(|effect| matches!(effect, Effect::CreatePage { page: hosted, .. } if hosted.as_str() == *page));
    assert!(hosted);
    session.app.undo();
    assert_eq!(node_ids(&session.canvas()), ["n1"]);
}

#[test]
fn a_tab_ref_naming_the_active_canvas_is_an_ordinary_write() {
    let mut session = space();
    let patch = json!({ "delete": ["p2"] });
    ok(session.call(&tabbed(Request::post("/canvas/apply", patch), "tab_1")));
    assert_eq!(node_ids(&session.canvas()), ["p1"]);
    assert!(session.app.app().can_undo());
    assert!(session.effects.contains(&Effect::Save));
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
