//! The `POST /canvas/apply` contract, replayed from the Electron app's
//! `tests/integration/canvas-apply.test.ts`: the same patch bodies, in the
//! same order, asserting what that suite asserts. Each test is named for
//! the case it replays.
//!
//! Where the two apps differ on purpose the test says so: deleting a group
//! here takes what is inside it (ADR 0034), and an edge to a missing entity
//! is refused.

use serde_json::{Value, json};

use crate::common::Scripted;

fn two_texts(session: &mut Scripted) -> (Value, Value) {
    let seeded = session.apply(json!({ "entities": [
        { "kind": "text", "text": "from", "canvasX": 0, "canvasY": 0 },
        { "kind": "text", "text": "to", "canvasX": 200, "canvasY": 0 },
    ]}));
    (seeded["created"][0].clone(), seeded["created"][1].clone())
}

#[test]
fn creates_entities_and_exposes_them_via_the_workspace_snapshot() {
    let mut session = Scripted::empty();
    let done = session.apply(json!({ "entities": [
        { "kind": "text", "text": "alpha", "canvasX": 0, "canvasY": 0 },
        { "kind": "text", "text": "beta", "canvasX": 200, "canvasY": 0 },
    ]}));
    assert_eq!(done["created"].as_array().map(Vec::len), Some(2));
    let alpha = session.node(&done["created"][0]);
    let beta = session.node(&done["created"][1]);
    assert_eq!(alpha["type"], "text");
    assert_eq!(alpha["text"], "alpha");
    assert_eq!(beta["text"], "beta");
    assert_eq!(beta["x"], 200);
}

#[test]
fn updates_an_entity_in_place() {
    let mut session = Scripted::empty();
    let id = session.apply(json!({ "entities": [
        { "kind": "text", "text": "before", "canvasX": 0, "canvasY": 0 },
    ]}))["created"][0]
        .clone();
    let done =
        session.apply(json!({ "entities": [{ "id": id, "kind": "text", "text": "after" }] }));
    assert_eq!(done["updated"], json!([id]));
    assert_eq!(session.node(&id)["text"], "after");
}

#[test]
fn updates_an_entity_passing_only_its_id_kind_resolves_from_the_doc() {
    let mut session = Scripted::empty();
    let id = session.apply(json!({ "entities": [
        { "kind": "text", "text": "before", "canvasX": 0, "canvasY": 0 },
    ]}))["created"][0]
        .clone();
    let done = session.apply(json!({ "entities": [{ "id": id, "text": "after" }] }));
    assert_eq!(done["updated"], json!([id]));
    assert_eq!(session.node(&id)["text"], "after");
}

#[test]
fn reports_nothing_deleted_for_an_id_the_doc_does_not_know() {
    let mut session = Scripted::empty();
    let done = session.apply(json!({ "delete": ["text_does_not_exist"] }));
    assert_eq!(done["deleted"], json!([]));
    assert!(
        !session.app.app().can_undo(),
        "a patch that did nothing is no undo step"
    );
}

#[test]
fn creates_edges_and_deletes_entities_by_id_resolving_kind_from_the_doc() {
    let mut session = Scripted::empty();
    let (from, to) = two_texts(&mut session);
    let linked = session.apply(json!({ "edges": [
        { "fromEntityId": from, "toEntityId": to, "kind": "connection", "label": "digest of" },
    ]}));
    assert_eq!(linked["edges"].as_array().map(Vec::len), Some(1));
    let canvas = session.canvas();
    let edge = &canvas["edges"][0];
    assert_eq!(edge["id"], linked["edges"][0]);
    assert_eq!(edge["label"], "digest of");
    assert_eq!(edge["edgeKind"], "connection");

    let removed = session.apply(json!({ "delete": [from] }));
    assert_eq!(removed["deleted"], json!([from]));
    assert_eq!(session.node(&from), Value::Null);
    assert_eq!(session.node(&to)["text"], "to");
    // The edge lost an end, so it went too.
    assert_eq!(session.canvas()["edges"], json!([]));
}

#[test]
fn patches_an_edge_in_place_when_the_apply_patch_reuses_its_id() {
    let mut session = Scripted::empty();
    let (from, to) = two_texts(&mut session);
    let linked = session.apply(json!({ "edges": [
        { "fromEntityId": from, "toEntityId": to, "kind": "connection", "label": "v1" },
    ]}));
    let edge = linked["edges"][0].clone();

    let relabeled = session.apply(json!({ "edges": [{ "id": edge, "label": "v2" }] }));
    assert_eq!(relabeled["edges"], json!([edge]));
    let edges = session.canvas()["edges"].clone();
    assert_eq!(edges.as_array().map(Vec::len), Some(1));
    assert_eq!(edges[0]["label"], "v2");

    session.undo();
    let edges = session.canvas()["edges"].clone();
    assert_eq!(edges.as_array().map(Vec::len), Some(1));
    assert_eq!(edges[0]["label"], "v1");
}

#[test]
fn creates_every_entity_kind_via_apply_incl_drawing_and_shape() {
    let mut session = Scripted::empty();
    let seed = session.apply(json!({ "entities": [
        { "kind": "text", "forceKind": true, "text": "a", "canvasX": 0, "canvasY": 0 },
        { "kind": "text", "forceKind": true, "text": "b", "canvasX": 200, "canvasY": 0 },
    ]}));
    let (a, b) = (&seed["created"][0], &seed["created"][1]);
    let done = session.apply(json!({ "entities": [
        { "kind": "page", "url": "https://example.com", "presetIndex": 9, "canvasX": 0, "canvasY": 400 },
        { "kind": "shape", "shapeKind": "rectangle", "text": "box", "canvasX": 400, "canvasY": 400 },
        { "kind": "drawing", "canvasX": 800, "canvasY": 400, "width": 100, "height": 100,
          "strokes": [{ "id": "s1", "color": "#000", "width": 2,
                        "points": [{ "x": 0, "y": 0 }, { "x": 50, "y": 50 }] }] },
        { "kind": "group", "entityIds": [a, b], "label": "pair" },
    ]}));
    let created = done["created"].as_array().cloned().unwrap_or_default();
    assert_eq!(created.len(), 4);
    let types: Vec<Value> = created
        .iter()
        .map(|id| session.node(id)["type"].clone())
        .collect();
    assert_eq!(types, ["link", "shape", "drawing", "group"]);
    // The page is sized by its preset, and its host is asked for.
    let page = session.node(&created[0]);
    assert_eq!(
        (&page["width"], &page["height"]),
        (&json!(466), &json!(678))
    );
    assert!(session.effects.iter().any(|effect| matches!(
        effect,
        specular_interact::Effect::CreatePage { url, .. } if url == "https://example.com"
    )));
    assert_eq!(session.node(&created[3])["label"], "pair");
    assert_eq!(session.node(a)["specular"]["parentGroupId"], created[3]);
}

#[test]
fn round_trips_shape_styling_and_undo_restores_the_prior_fill_border_and_text_alignment() {
    let mut session = Scripted::empty();
    let id = session.apply(json!({ "entities": [{
        "kind": "shape", "shapeKind": "rectangle", "canvasX": 0, "canvasY": 0,
        "fillStyle": "solid", "borderStyle": "solid", "borderColor": "4", "strokeWidth": 3,
        "textAlign": "center", "textVerticalAlign": "middle",
    }]}))["created"][0]
        .clone();
    let styling = |node: &Value| {
        json!([
            node["specular"]["fillStyle"],
            node["specular"]["textAlign"],
            node["specular"]["textVerticalAlign"],
            node["borderStyle"],
            node["borderColor"],
            node["strokeWidth"],
        ])
    };
    let seeded = json!(["solid", "center", "middle", "solid", "4", 3]);
    assert_eq!(styling(&session.node(&id)), seeded);

    session.apply(json!({ "entities": [{
        "id": id, "kind": "shape", "fillStyle": "none", "borderStyle": "none", "borderColor": "1",
        "strokeWidth": 1, "textAlign": "right", "textVerticalAlign": "bottom",
    }]}));
    assert_eq!(
        styling(&session.node(&id)),
        json!(["none", "right", "bottom", "none", "1", 1])
    );

    session.undo();
    assert_eq!(styling(&session.node(&id)), seeded);
}

#[test]
fn renames_a_group_via_text_aliased_to_label_explicit_label_wins_undo_restores_the_prior_name() {
    let mut session = Scripted::empty();
    let (a, b) = two_texts(&mut session);
    let group = session.apply(json!({ "entities": [
        { "kind": "group", "entityIds": [a, b], "label": "original" },
    ]}))["created"][0]
        .clone();

    let done =
        session.apply(json!({ "entities": [{ "id": group, "kind": "group", "text": "renamed" }] }));
    assert_eq!(done["updated"], json!([group]));
    assert_eq!(session.node(&group)["label"], "renamed");

    session.undo();
    assert_eq!(session.node(&group)["label"], "original");

    session.apply(json!({ "entities": [
        { "id": group, "kind": "group", "text": "ignored", "label": "explicit" },
    ]}));
    assert_eq!(session.node(&group)["label"], "explicit");
}

#[test]
fn accepts_kind_note_as_an_alias_for_text() {
    let mut session = Scripted::empty();
    let done = session.apply(json!({ "entities": [
        { "kind": "note", "text": "short note", "canvasX": 0, "canvasY": 0 },
    ]}));
    assert_eq!(session.node(&done["created"][0])["type"], "text");
}

#[test]
fn applies_create_and_delete_as_one_patch() {
    let mut session = Scripted::empty();
    let seed = session.apply(json!({ "entities": [
        { "kind": "text", "text": "seed", "canvasX": 0, "canvasY": 0 },
    ]}))["created"][0]
        .clone();
    let done = session.apply(json!({
        "entities": [
            { "kind": "text", "text": "one", "canvasX": 0, "canvasY": 200 },
            { "kind": "text", "text": "two", "canvasX": 200, "canvasY": 200 },
        ],
        "delete": [seed],
    }));
    assert_eq!(done["created"].as_array().map(Vec::len), Some(2));
    assert_eq!(done["deleted"], json!([seed]));
    let canvas = session.canvas();
    let ids: Vec<&Value> = canvas["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| &n["id"])
        .collect();
    assert_eq!(ids, [&done["created"][0], &done["created"][1]]);
}

#[test]
fn throws_canvas_patch_error_before_mutating_anything_on_a_bad_item() {
    let mut session = Scripted::empty();
    let response = session.post(
        "/canvas/apply",
        json!({ "entities": [
            { "kind": "text", "text": "good", "canvasX": 0, "canvasY": 0 },
            { "kind": "not-a-kind", "canvasX": 200, "canvasY": 0 },
        ]}),
    );
    assert_eq!(response.status, 400);
    assert_eq!(
        response.body,
        json!({ "error": "entities[1]: missing or unknown kind" })
    );
    // The good item must not have leaked through.
    assert_eq!(session.canvas()["nodes"], json!([]));
}

#[test]
fn a_multi_item_patch_collapses_to_one_undo_step() {
    let mut session = Scripted::empty();
    let done = session.apply(json!({ "entities": [
        { "kind": "text", "text": "one", "canvasX": 0, "canvasY": 0 },
        { "kind": "text", "text": "two", "canvasX": 200, "canvasY": 0 },
        { "kind": "text", "text": "three", "canvasX": 400, "canvasY": 0 },
    ]}));
    assert_eq!(done["created"].as_array().map(Vec::len), Some(3));
    session.undo();
    assert_eq!(session.canvas()["nodes"], json!([]));
    session.app.assert_undo_returns_to_start();
}

#[test]
fn moving_a_drawing_via_update_carries_its_strokes_and_undo_restores_them() {
    let mut session = Scripted::empty();
    let id = session.apply(json!({ "entities": [{
        "kind": "drawing", "canvasX": 0, "canvasY": 0, "width": 100, "height": 100,
        "strokes": [{ "id": "s", "color": "#000", "width": 2, "points": [{ "x": 10, "y": 20 }] }],
    }]}))["created"][0]
        .clone();

    session.apply(
        json!({ "entities": [{ "id": id, "kind": "drawing", "canvasX": 300, "canvasY": 400 }] }),
    );
    let moved = session.node(&id);
    assert_eq!((&moved["x"], &moved["y"]), (&json!(300), &json!(400)));
    assert_eq!(
        moved["strokes"][0]["points"][0],
        json!({ "x": 310, "y": 420 })
    );

    session.undo();
    let back = session.node(&id);
    assert_eq!((&back["x"], &back["y"]), (&json!(0), &json!(0)));
    assert_eq!(back["strokes"][0]["points"][0], json!({ "x": 10, "y": 20 }));
}

#[test]
fn moving_a_group_via_update_carries_its_children_as_one_undo_step() {
    let mut session = Scripted::empty();
    let seed = session.apply(json!({ "entities": [
        { "kind": "text", "text": "a", "canvasX": 0, "canvasY": 0 },
        { "kind": "text", "text": "b", "canvasX": 40, "canvasY": 0 },
    ]}));
    let (a, b) = (seed["created"][0].clone(), seed["created"][1].clone());
    let group = session.apply(json!({ "entities": [
        { "kind": "group", "entityIds": [a, b], "label": "g" },
    ]}))["created"][0]
        .clone();
    let (g0, a0) = (session.node(&group), session.node(&a));
    let coordinate = |node: &Value, key: &str| node[key].as_f64().unwrap_or(f64::NAN);

    session.apply(json!({ "entities": [{
        "id": group, "kind": "group",
        "canvasX": coordinate(&g0, "x") + 100.0, "canvasY": coordinate(&g0, "y") + 50.0,
    }]}));
    let a1 = session.node(&a);
    assert_eq!(coordinate(&a1, "x"), coordinate(&a0, "x") + 100.0);
    assert_eq!(coordinate(&a1, "y"), coordinate(&a0, "y") + 50.0);
    assert_eq!(
        coordinate(&session.node(&group), "x"),
        coordinate(&g0, "x") + 100.0
    );

    session.undo();
    assert_eq!(session.node(&a), a0);
    assert_eq!(session.node(&group), g0);
}
