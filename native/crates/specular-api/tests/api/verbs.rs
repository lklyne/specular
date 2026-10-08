//! The verbs end to end: a request through the planned event and `update`,
//! then the response and the document.

use serde_json::{Value, json};
use specular_doc::Rect;
use specular_testkit::{TestApp, assert_doc_snapshot, connected, document, sticky};

use crate::common::Scripted;

fn three_notes() -> Scripted {
    let mut app = TestApp::from_document(connected(
        document([
            sticky("a", Rect::new(0.0, 0.0, 200.0, 200.0), "one"),
            sticky("b", Rect::new(300.0, 0.0, 200.0, 200.0), "two"),
            sticky("c", Rect::new(600.0, 0.0, 200.0, 200.0), "three"),
        ]),
        "e1",
        "a",
        "b",
    ));
    app.viewport((1000.0, 800.0));
    Scripted::new(app)
}

#[track_caller]
fn ok(session: &mut Scripted, path: &str, body: Value) -> Value {
    let response = session.post(path, body);
    assert_eq!(response.status, 200, "{path}: {}", response.body);
    response.body
}

#[test]
fn select_then_read_the_selection() {
    let mut session = three_notes();
    let done = ok(
        &mut session,
        "/selection/select-entities",
        json!({ "entityIds": ["a", "c", "e1"] }),
    );
    let selection = json!({
        "selectedEntityId": "a", "selectedEntityIds": ["a", "c"], "selectedEdgeIds": ["e1"],
    });
    assert_eq!(done, json!({ "ok": true, "selection": selection }));
    assert_eq!(session.get("/selection").body, selection);
    // An id that names nothing is dropped and reported.
    let partial = ok(
        &mut session,
        "/selection/select-entities",
        json!({ "entityIds": ["b", "x"] }),
    );
    assert_eq!(
        partial,
        json!({ "ok": false, "selection": { "selectedEntityId": "b", "selectedEntityIds": ["b"] } })
    );
    assert_eq!(
        ok(&mut session, "/selection/deselect", json!({})),
        json!({ "ok": true, "selection": {} })
    );
}

#[test]
fn restack_group_ungroup_and_undo_are_each_one_step() {
    let mut session = three_notes();
    let order = ok(
        &mut session,
        "/stack-order/send-to-back",
        json!({ "id": "c" }),
    );
    assert_eq!(
        order,
        json!({ "ok": true, "entityOrder": ["c", "a", "b", "e1"] })
    );

    let group = ok(
        &mut session,
        "/groups/create",
        json!({ "entityIds": ["a", "b"], "label": "pair" }),
    );
    let id = group["id"].clone();
    assert_eq!(group["entityIds"], json!(["a", "b"]));
    let restacked = json!({ "selectedEntityId": "c", "selectedEntityIds": ["c"] });
    assert_eq!(
        session.get("/selection").body,
        restacked,
        "a patch leaves the selection alone"
    );
    assert_eq!(session.node(&id)["label"], "pair");
    // 24 units of room round its members.
    let node = session.node(&id);
    assert_eq!(
        json!([node["x"], node["y"], node["width"], node["height"]]),
        json!([-24, -24, 548, 248])
    );

    let freed = ok(&mut session, "/groups/ungroup", json!({ "groupId": id }));
    assert_eq!(freed, json!({ "entityIds": ["a", "b"] }));
    assert_eq!(session.node(&id), Value::Null);

    // Three steps, and the history walks back through each.
    let undone = ok(&mut session, "/history/undo", json!({}));
    assert_eq!(
        undone,
        json!({ "ok": true, "canUndo": true, "canRedo": true })
    );
    assert_eq!(session.node(&id)["label"], "pair");
    session.undo();
    session.undo();
    assert_eq!(ok(&mut session, "/history/undo", json!({}))["ok"], false);
    let redone = ok(&mut session, "/history/redo", json!({}));
    assert_eq!(
        redone,
        json!({ "ok": true, "canUndo": true, "canRedo": true })
    );
    session.app.assert_undo_returns_to_start();
}

#[test]
fn focus_fits_the_named_entities_and_selects_the_first() {
    let mut session = three_notes();
    assert_eq!(
        ok(
            &mut session,
            "/camera/focus",
            json!({ "pageIds": ["b", "c"] })
        ),
        json!({ "focused": true })
    );
    let camera = session.app.session().camera;
    // 500 wide from x=300: centred in 1000 at 100%.
    assert_eq!(
        (camera.zoom, camera.pan.x, camera.pan.y),
        (1.0, -50.0, 300.0)
    );
    assert_eq!(session.app.selected_ids(), ["b"]);
    let canvas = session.canvas();
    assert_eq!(canvas["appState"]["pan"], json!({ "x": -50, "y": 300 }));
}

#[test]
fn a_write_is_refused_while_a_drag_is_in_flight() {
    let mut session = three_notes();
    session.app.press((100.0, 100.0)).drag_to((160.0, 100.0));
    let response = session.post(
        "/canvas/apply",
        json!({ "entities": [{ "id": "c", "text": "x" }] }),
    );
    assert_eq!(response.status, 409);
    assert_eq!(
        response.body,
        json!({ "error": "a drag is in flight; try again when it ends" })
    );
    session.app.release();
    session.apply(json!({ "entities": [{ "id": "c", "text": "x" }] }));
    assert_eq!(session.node(&json!("c"))["text"], "x");
}

#[test]
fn comments_are_created_answered_and_listed() {
    let mut session = three_notes();
    let made = ok(
        &mut session,
        "/annotations",
        json!({
            "text": "tighten this", "anchor": { "type": "viewport" }, "author": "agent",
        }),
    );
    let id = made["id"].as_str().unwrap_or_default().to_owned();
    assert!(id.starts_with("ann_"), "{id}");
    // The middle of a 1000x800 viewport at the default camera.
    assert_eq!(
        made["anchor"],
        json!({ "type": "canvas", "canvasX": 500, "canvasY": 400 })
    );
    assert_eq!(
        (&made["author"], &made["status"]),
        (&json!("agent"), &json!("pending"))
    );

    let acked = ok(
        &mut session,
        &format!("/annotations/{id}/acknowledge"),
        json!({}),
    );
    assert_eq!(acked["status"], "acknowledged");
    let replied = ok(
        &mut session,
        &format!("/annotations/{id}/reply"),
        json!({ "author": "agent", "text": "done" }),
    );
    assert_eq!(replied["replies"][0]["text"], "done");
    let dismissed = ok(
        &mut session,
        &format!("/annotations/{id}/dismiss"),
        json!({ "reason": "dup" }),
    );
    assert_eq!(dismissed["metadata"], json!({ "dismissReason": "dup" }));

    let count = |session: &mut Scripted, query: &str| {
        session.get(&format!("/annotations{query}")).body["annotations"]
            .as_array()
            .map(Vec::len)
    };
    assert_eq!(count(&mut session, "?status=unresolved"), Some(0));
    assert_eq!(count(&mut session, "?status=dismissed"), Some(1));
    assert_eq!(count(&mut session, "?status=all"), Some(1));

    let resolved = ok(
        &mut session,
        &format!("/annotations/{id}/resolve"),
        json!({}),
    );
    assert_eq!(
        resolved.get("metadata"),
        None,
        "the reason leaves with the dismissal"
    );
    assert_eq!(
        session.delete(&format!("/annotations/{id}")).body,
        json!({ "ok": true })
    );
    assert_eq!(session.get(&format!("/annotations/{id}")).status, 404);
    assert_eq!(
        session.post("/annotations", json!({ "text": "x" })).status,
        400
    );
    session.app.assert_undo_returns_to_start();
}

#[test]
fn a_selection_comment_carries_what_it_is_about() {
    let mut session = three_notes();
    session.app.select(&["a", "b"]);
    let made = ok(
        &mut session,
        "/selection/annotate",
        json!({ "text": " compare these " }),
    );
    assert_eq!(
        made["anchor"],
        json!({ "type": "region", "canvasRect": { "x": 0, "y": 0, "width": 500, "height": 200 } })
    );
    assert_eq!(made["selectionEntityIds"], json!(["a", "b"]));
    let id = made["id"].as_str().unwrap_or_default().to_owned();
    let detail = session.get(&format!("/annotations/{id}")).body;
    assert_eq!(detail["text"], "compare these");
    let texts: Vec<&Value> = detail["selection"]["members"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|m| &m["text"])
        .collect();
    assert_eq!(texts, ["one", "two"]);

    let named = ok(
        &mut session,
        "/selection/annotate",
        json!({ "text": "this one", "entityIds": ["c"] }),
    );
    assert_eq!(named["selectionEntityIds"], json!(["c"]));
    session.app.select(&[]);
    let nothing = session.post("/selection/annotate", json!({ "text": "x" }));
    assert_eq!(
        (nothing.status, &nothing.body["error"]),
        (400, &json!("No entities selected"))
    );
}

#[test]
fn new_things_are_placed_clear_of_what_is_there() {
    let mut session = three_notes();
    // Nothing selected: the first free spot from the top-left, a gutter
    // clear of the three notes along the top.
    let spot = ok(
        &mut session,
        "/layout/find-placement",
        json!({ "width": 200, "height": 200 }),
    );
    assert_eq!(
        spot,
        json!({ "canvasX": 880, "canvasY": 80, "fallbackUsed": true, "reason": "scan_fit" })
    );
    // Beside the selection when there is one and the spot is free.
    session.app.select(&["c"]);
    let beside = ok(
        &mut session,
        "/layout/find-placement",
        json!({ "width": 200, "height": 200 }),
    );
    assert_eq!(
        beside,
        json!({ "canvasX": 880, "canvasY": 0, "fallbackUsed": false, "reason": "selection_anchor" })
    );

    let batch = ok(
        &mut session,
        "/layout/batch-placement",
        json!({
            "items": [{ "width": 200, "height": 200 }, { "width": 1304, "height": 824, "insetX": 12, "insetY": 12 }],
        }),
    );
    assert_eq!(
        batch["positions"],
        json!([
            { "canvasX": 880, "canvasY": 0 }, { "canvasX": 1172, "canvasY": 12 },
        ])
    );

    let grid = ok(
        &mut session,
        "/layout/apply-directive",
        json!({
            "layout": { "kind": "grid", "cols": 2, "gap": "xs" },
            "items": [{ "id": "c" }, { "id": "a" }, { "id": "b" }],
        }),
    );
    assert_eq!(grid["kinds"], json!(["text", "text", "text"]));
    assert_eq!(
        grid["positions"],
        json!([
            { "canvasX": 0, "canvasY": 0 }, { "canvasX": 220, "canvasY": 0 }, { "canvasX": 0, "canvasY": 220 },
        ])
    );
    let bad = session.post(
        "/layout/apply-directive",
        json!({ "layout": { "kind": "pile" }, "items": [] }),
    );
    assert_eq!(
        bad.body,
        json!({ "error": "layout.kind: expected 'row' | 'column' | 'grid', got \"pile\"" })
    );
}

#[test]
fn a_note_grows_to_hold_its_text_and_keeps_a_size_it_fits_in() {
    let mut session = Scripted::empty();
    let long = "word ".repeat(200);
    let id = session.apply(json!({ "entities": [{ "kind": "text", "text": long }] }))["created"][0]
        .clone();
    let grown = session.node(&id)["height"].as_f64().unwrap_or_default();
    assert!(grown > 200.0, "a sticky as tall as its text, not {grown}");
    assert_eq!(session.node(&id)["width"], 200);

    // Shorter text leaves it as tall as it was, and a taller size is kept.
    session.apply(json!({ "entities": [{ "id": id, "text": "short" }] }));
    assert_eq!(session.node(&id)["height"], grown);
    session.apply(json!({ "entities": [{ "id": id, "height": grown + 100.0 }] }));
    assert_eq!(session.node(&id)["height"], grown + 100.0);
    session.app.assert_undo_returns_to_start();
}

#[test]
fn a_session_of_verbs_builds_a_canvas() {
    let mut session = Scripted::empty();
    let made = session.apply(json!({ "entities": [
        { "kind": "page", "url": "http://localhost:4321/garden", "canvasX": 0, "canvasY": 0 },
        { "kind": "page", "url": "http://localhost:4321/garden", "presetIndex": 1, "canvasX": 1400, "canvasY": 0 },
        { "kind": "text", "text": "hero is too tall", "canvasX": 0, "canvasY": 900, "color": "4" },
        { "kind": "shape", "shapeKind": "ellipse", "text": "todo", "canvasX": 300, "canvasY": 900 },
    ]}));
    let ids = made["created"].clone();
    session.apply(json!({ "edges": [
        { "fromEntityId": ids[0], "toEntityId": ids[1], "kind": "breakpoint_variant" },
        { "fromEntityId": ids[2], "toEntityId": ids[0], "label": "about" },
    ]}));
    session.apply(json!({ "entities": [
        { "kind": "group", "entityIds": [ids[2], ids[3]], "label": "notes" },
        { "id": ids[1], "orientation": "landscape" },
    ]}));
    // The group follows a member moved inside it.
    session.apply(json!({ "entities": [{ "id": ids[3], "canvasX": 320, "color": "neutral" }] }));
    assert_doc_snapshot!("a_session_of_verbs_builds_a_canvas", session.app);
    session.app.assert_undo_returns_to_start();
}
