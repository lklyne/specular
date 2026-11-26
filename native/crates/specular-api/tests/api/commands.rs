//! `POST /run` and `GET /commands`: the command registry over HTTP, which
//! the CLI's `run` and `commands` verbs call.

use serde_json::{Value, json};
use specular_doc::Rect;
use specular_testkit::{TestApp, document, sticky};

use crate::common::Scripted;

fn two_notes() -> Scripted {
    let mut app = TestApp::from_document(document([
        sticky("a", Rect::new(0.0, 0.0, 200.0, 200.0), "one"),
        sticky("b", Rect::new(300.0, 0.0, 200.0, 200.0), "two"),
    ]));
    app.viewport((1000.0, 800.0));
    Scripted::new(app)
}

#[track_caller]
fn ok(session: &mut Scripted, body: Value) -> Value {
    let response = session.post("/run", body);
    assert_eq!(response.status, 200, "{}", response.body);
    response.body
}

#[test]
fn run_runs_a_command_by_name_as_one_step_and_commands_lists_them() {
    let mut session = two_notes();
    let listed = session.get("/commands").body;
    let undo = (listed["commands"].as_array().into_iter().flatten())
        .find(|command| command["name"] == "edit.undo")
        .cloned()
        .unwrap_or_default();
    assert_eq!(undo["enabled"], false, "{undo}");
    let select = json!({ "name": "edit.select", "args": { "ids": ["a", "b"] } });
    ok(&mut session, select);
    let done = ok(
        &mut session,
        json!({ "name": "edit.nudge", "args": { "dx": 10, "dy": 0 } }),
    );
    assert_eq!(done, json!({ "ok": true, "name": "edit.nudge" }));
    assert_eq!(session.node(&json!("b"))["x"], 310);
    let refused = [
        (
            json!({ "name": "edit.nudj" }),
            404,
            "did you mean edit.nudge",
        ),
        (json!({ "name": "edit.nudge" }), 400, "`dx` is a number"),
        (json!({ "name": "edit.redo" }), 409, "cannot run now"),
        (json!({ "name": "app.quit" }), 409, "the window's own"),
    ];
    for (body, status, text) in refused {
        let response = session.post("/run", body.clone());
        assert_eq!(response.status, status, "{body}: {}", response.body);
        assert!(
            response.body.to_string().contains(text),
            "{body}: {}",
            response.body
        );
    }
    ok(&mut session, json!({ "name": "edit.undo" }));
    assert_eq!(session.node(&json!("b"))["x"], 300);
}
