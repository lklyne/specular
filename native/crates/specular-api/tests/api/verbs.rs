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
    let none = ok(
        &mut session,
        "/selection/select-entities",
        json!({ "entityIds": ["x"] }),
    );
    assert_eq!(none, json!({ "ok": false, "selection": {} }));
    let empty = ok(
        &mut session,
        "/selection/select-entities",
        json!({ "entityIds": [] }),
    );
    assert_eq!(empty, json!({ "ok": false, "selection": {} }));
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

    // An id that names nothing is skipped when picking what to select.
    session.app.select(&[]);
    ok(
        &mut session,
        "/camera/focus",
        json!({ "pageIds": ["ghost", "c"] }),
    );
    assert_eq!(session.app.selected_ids(), ["c"]);
    // Bounds given by hand are looked at, and nothing is selected.
    session.app.select(&[]);
    ok(
        &mut session,
        "/camera/focus",
        json!({ "pageIds": ["a"], "bounds": { "x": 0, "y": 0, "width": 500, "height": 400 } }),
    );
    assert_eq!(session.app.selected_ids(), [] as [&str; 0]);
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
    let anonymous = ok(
        &mut session,
        "/annotations",
        json!({ "text": "mine", "anchor": { "type": "viewport" } }),
    );
    assert_eq!(anonymous["author"], "user");
    let anonymous_id = anonymous["id"].as_str().unwrap_or_default();
    assert_eq!(
        session
            .delete(&format!("/annotations/{anonymous_id}"))
            .status,
        200
    );
    for missing in [
        json!({ "anchor": { "type": "viewport" } }),
        json!({ "text": "no anchor" }),
    ] {
        assert_eq!(session.post("/annotations", missing).status, 400);
    }

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
    let unsigned = ok(
        &mut session,
        &format!("/annotations/{id}/reply"),
        json!({ "text": "again" }),
    );
    assert_eq!(unsigned["replies"][1]["author"], "agent");
    let empty = session.post(&format!("/annotations/{id}/reply"), json!({ "text": "" }));
    assert_eq!(empty.status, 400, "{}", empty.body);
    let bare = ok(
        &mut session,
        &format!("/annotations/{id}/dismiss"),
        json!({ "reason": "" }),
    );
    assert_eq!(bare.get("metadata"), None, "an empty reason is no reason");
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
fn comments_on_a_page_are_listed_by_page_and_address_and_name_what_they_point_at() {
    let mut session = Scripted::empty();
    let made = session.apply(json!({ "entities": [
        { "kind": "page", "url": "https://example.com/a/", "canvasX": 0, "canvasY": 0 },
        { "kind": "page", "url": "https://example.com/b", "canvasX": 1400, "canvasY": 0 },
    ]}));
    let (one, two) = (made["created"][0].clone(), made["created"][1].clone());
    // No offsets are given: a page anchor defaults them to the corner.
    for page in [&one, &two] {
        ok(
            &mut session,
            "/annotations",
            json!({ "text": "here", "anchor": { "type": "page", "pageId": page } }),
        );
    }
    let ghost = session.post(
        "/annotations",
        json!({ "text": "x", "anchor": { "type": "page", "pageId": "ghost" } }),
    );
    assert_eq!(ghost.status, 404, "{}", ghost.body);

    let found = |session: &mut Scripted, query: &str| -> Vec<Value> {
        let body = session.get(&format!("/annotations{query}")).body;
        body["annotations"].as_array().cloned().unwrap_or_default()
    };
    let by_page = found(
        &mut session,
        &format!("?page_id={}", one.as_str().unwrap_or_default()),
    );
    assert_eq!(by_page.len(), 1);
    assert_eq!(
        by_page[0]["pageAnchor"]["pageUrl"],
        "https://example.com/a/"
    );
    // A trailing slash on either side is the same address.
    assert_eq!(
        found(&mut session, "?url=https%3A%2F%2Fexample.com%2Fa").len(),
        1
    );
    assert_eq!(
        found(&mut session, "?url=https%3A%2F%2Fexample.com%2Fb%2F").len(),
        1
    );
    assert_eq!(
        found(&mut session, "?url=https%3A%2F%2Fexample.com%2Fnope").len(),
        0
    );
    assert_eq!(found(&mut session, "").len(), 2);

    // A comment on one page or one file names it; the detail lists the members.
    session.disk.files.insert(
        "/space/a.png".to_owned(),
        specular_interact::DroppedFile {
            path: "/space/a.png".to_owned(),
            space_path: Some("a.png".to_owned()),
            image_size: Some((10, 10)),
        },
    );
    let file = session.apply(json!({ "entities": [{ "kind": "file", "file": "/space/a.png" }] }))
        ["created"][0]
        .clone();
    let on_page = ok(
        &mut session,
        "/selection/annotate",
        json!({ "text": "p", "entityIds": [one] }),
    );
    assert_eq!(
        on_page["selectionTarget"],
        json!({ "entityId": one, "kind": "page", "url": "https://example.com/a/" })
    );
    let on_file = ok(
        &mut session,
        "/selection/annotate",
        json!({ "text": "f", "entityIds": [file] }),
    );
    assert_eq!(
        on_file["selectionTarget"],
        json!({ "entityId": file, "kind": "file", "filePath": "a.png" })
    );
    let id = on_page["id"].as_str().unwrap_or_default();
    let detail = session.get(&format!("/annotations/{id}")).body;
    let member = &detail["selection"]["members"][0];
    assert_eq!(
        (&member["url"], &member["kind"]),
        (&json!("https://example.com/a/"), &json!("page"))
    );
    let id = on_file["id"].as_str().unwrap_or_default();
    let detail = session.get(&format!("/annotations/{id}")).body;
    assert_eq!(detail["selection"]["members"][0]["filePath"], "a.png");
    // A region comment that already holds its picture is not photographed again.
    let region =
        json!({ "type": "region", "canvasRect": { "x": 0, "y": 0, "width": 50, "height": 50 } });
    let kept = ok(
        &mut session,
        "/annotations",
        json!({ "text": "r", "anchor": region, "metadata": { "regionScreenshot": "kept" } }),
    );
    let taken = session.shots.len();
    let id = kept["id"].as_str().unwrap_or_default();
    let detail = session.get(&format!("/annotations/{id}")).body;
    assert_eq!(detail["metadata"]["regionScreenshot"], "kept");
    assert_eq!(session.shots.len(), taken);
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
    // A region comment comes with a picture of its region as it is now.
    assert_eq!(detail["metadata"]["regionScreenshot"], "cGl4ZWxz");
    let region = specular_doc::Rect::new(0.0, 0.0, 500.0, 200.0);
    assert_eq!(
        session.shots[0].area,
        specular_api::ShotArea::Canvas(region)
    );

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
#[expect(clippy::too_many_lines, reason = "one session of placements")]
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
            "layout": { "kind": "grid", "cols": 3, "gap": "xs" },
            "items": [{ "id": "c" }, { "id": "a" }, { "id": "b" }],
        }),
    );
    assert_eq!(grid["kinds"], json!(["text", "text", "text"]));
    assert_eq!(
        grid["positions"],
        json!([
            { "canvasX": 0, "canvasY": 0 }, { "canvasX": 220, "canvasY": 0 }, { "canvasX": 440, "canvasY": 0 },
        ])
    );
    // Item sizes and the gap snap to the 20-unit grid.
    let snapped = ok(
        &mut session,
        "/layout/batch-placement",
        json!({ "items": [{ "width": 190, "height": 190 }, { "width": 190, "height": 190 }], "gap": 30 }),
    );
    assert_eq!(
        snapped["positions"],
        json!([{ "canvasX": 880, "canvasY": 0 }, { "canvasX": 1120, "canvasY": 0 }])
    );
    let directive = |session: &mut Scripted, layout: Value, items: Value| {
        ok(
            session,
            "/layout/apply-directive",
            json!({ "layout": layout, "items": items }),
        )["positions"]
            .clone()
    };
    // An origin counts from the first item's inset.
    assert_eq!(
        directive(
            &mut session,
            json!({ "kind": "row", "originX": 100, "originY": 100, "gap": 20 }),
            json!([{ "width": 50, "height": 50, "insetX": 10, "insetY": 5 }, { "width": 50, "height": 50 }]),
        ),
        json!([{ "canvasX": 100, "canvasY": 100 }, { "canvasX": 160, "canvasY": 95 }])
    );
    // Beside a named entity: a column goes below it, a row to its right.
    assert_eq!(
        directive(
            &mut session,
            json!({ "kind": "column", "near": "a", "gap": 20 }),
            json!([{ "width": 50, "height": 50 }])
        ),
        json!([{ "canvasX": 0, "canvasY": 220 }])
    );
    assert_eq!(
        directive(
            &mut session,
            json!({ "kind": "row", "near": "a", "gap": 20 }),
            json!([{ "width": 50, "height": 50 }])
        ),
        json!([{ "canvasX": 220, "canvasY": 0 }])
    );
    // Row and column gaps are their own, and a size given for an entity wins.
    assert_eq!(
        directive(
            &mut session,
            json!({ "kind": "grid", "cols": 2, "colGap": "xs", "rowGap": "l" }),
            json!([{ "id": "a", "width": 100 }, { "id": "b", "width": 100 }, { "id": "c", "width": 100 }]),
        ),
        json!([{ "canvasX": 0, "canvasY": 0 }, { "canvasX": 120, "canvasY": 0 }, { "canvasX": 0, "canvasY": 300 }])
    );
    // New items with nothing to go on land beside the selection.
    session.app.select(&["c"]);
    assert_eq!(
        directive(
            &mut session,
            json!({ "kind": "row" }),
            json!([{ "width": 200, "height": 200 }])
        ),
        json!([{ "canvasX": 880, "canvasY": 0 }])
    );
    for (layout, error) in [
        (json!("row"), "layout: expected an object"),
        (
            json!({ "kind": "row", "originX": "a", "originY": 1 }),
            "layout.originX: expected number, got \"a\"",
        ),
        (
            json!({ "kind": "row", "near": 4 }),
            "layout.near: expected entity id string, got 4",
        ),
        (
            json!({ "kind": "grid", "cols": 0 }),
            "layout.cols: expected positive integer, got 0",
        ),
    ] {
        let refused = session.post(
            "/layout/apply-directive",
            json!({ "layout": layout, "items": [] }),
        );
        assert_eq!(refused.body, json!({ "error": error }), "{layout}");
    }
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
    let long = "word ".repeat(50);
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
    // A size too small for the text is grown to hold it.
    session.apply(json!({ "entities": [{ "id": id, "height": 40 }] }));
    assert_eq!(session.node(&id)["height"], 200);
    session.app.assert_undo_returns_to_start();
}

#[test]
fn plain_text_is_sized_to_its_words_and_a_write_ends_the_open_text_edit() {
    let mut session = three_notes();
    let made = session.apply(json!({ "entities": [
        { "kind": "text", "textStyle": "plain", "text": "hi", "canvasX": 0, "canvasY": 400 },
    ]}));
    let node = session.node(&made["created"][0]);
    assert_eq!(
        (node["width"].clone(), node["height"].clone()),
        (json!(64), json!(20))
    );

    session.app.double_click((100.0, 100.0));
    session.app.type_text("!");
    assert_ne!(session.app.editing_text(), "one");
    session.apply(json!({ "entities": [{ "id": "c", "text": "x" }] }));
    assert!(session.app.app().text_edit().is_none());
    assert!(
        session.node(&json!("a"))["text"]
            .as_str()
            .is_some_and(|text| text.contains('!'))
    );
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

#[test]
fn arrange_tidies_in_place_or_packs_at_a_gap_and_each_is_one_step() {
    // (the body, where a, b and c land along x).
    let rows = [
        // The footprint is kept: the ends stay and the middle evens out.
        (json!({ "mode": "row", "entityIds": ["a", "b", "c"] }), None),
        // A gap packs from the top-left.
        (
            json!({ "mode": "row", "entityIds": ["c", "a", "b"], "gap": 20 }),
            Some([0.0, 220.0, 440.0]),
        ),
        (
            json!({ "mode": "column", "entityIds": ["a", "b", "c"], "gap": "m" }),
            Some([0.0, 0.0, 0.0]),
        ),
    ];
    for (body, want) in rows {
        let mut session = three_notes();
        let done = ok(&mut session, "/selection/arrange", body.clone());
        let lefts = ["a", "b", "c"].map(|id| session.app.rect(id).x);
        match want {
            // Already even: nothing to do, and nothing to undo.
            None => {
                assert_eq!(done, json!({ "changed": false }), "{body}");
                assert!(!session.app.app().can_undo(), "{body}");
            }
            Some(want) => {
                assert_eq!(done, json!({ "changed": true }), "{body}");
                assert_eq!(lefts, want, "{body}");
                session.app.assert_undo_returns_to_start();
            }
        }
    }
    // A grid of three columns lays three notes in one row.
    let mut session = three_notes();
    ok(
        &mut session,
        "/selection/arrange",
        json!({ "mode": "grid", "entityIds": ["a", "b", "c"], "gap": 20, "cols": 3 }),
    );
    let at = ["a", "b", "c"].map(|id| (session.app.rect(id).x, session.app.rect(id).y));
    assert_eq!(at, [(0.0, 0.0), (220.0, 0.0), (440.0, 0.0)]);
    // Notes a little off the line still read as one row, in x order.
    let mut app = TestApp::from_document(document([
        sticky("a", Rect::new(0.0, 60.0, 200.0, 200.0), "one"),
        sticky("b", Rect::new(300.0, 0.0, 200.0, 200.0), "two"),
        sticky("c", Rect::new(600.0, 30.0, 200.0, 200.0), "three"),
    ]));
    app.viewport((1000.0, 800.0));
    let mut ragged = Scripted::new(app);
    ok(
        &mut ragged,
        "/selection/arrange",
        json!({ "mode": "row", "entityIds": ["a", "b", "c"], "gap": 20 }),
    );
    assert_eq!(
        ["a", "b", "c"].map(|id| ragged.app.rect(id).x),
        [0.0, 220.0, 440.0]
    );
    let mut session = three_notes();
    let bad = session.post("/selection/arrange", json!({ "mode": "pile" }));
    assert_eq!(bad.status, 400, "{}", bad.body);
}

#[test]
fn auto_layout_makes_a_managed_group_and_reorder_child_moves_a_member() {
    let mut session = three_notes();
    let made = ok(
        &mut session,
        "/groups/auto-layout",
        json!({ "entityIds": ["a", "b", "c"], "gap": 40, "label": "Row" }),
    );
    let id = made["id"].as_str().unwrap_or_default().to_owned();
    assert_eq!(
        made,
        json!({
            "id": id, "label": "Row", "canvasX": -24, "canvasY": -24,
            "width": 728, "height": 248, "layoutMode": "row", "managedLayout": true,
            "layoutGap": 40, "entityIds": ["a", "b", "c"],
        })
    );
    assert_eq!(
        ["a", "b", "c"].map(|id| session.app.rect(id).x),
        [0.0, 240.0, 480.0]
    );
    assert_eq!(
        session.get("/selection").body,
        json!({ "selectedGroupId": id })
    );

    let moved = ok(
        &mut session,
        "/groups/reorder-child",
        json!({ "groupId": id, "childId": "c", "toIndex": 0 }),
    );
    assert_eq!(moved, json!({ "changed": true }));
    assert_eq!(
        ["c", "a", "b"].map(|id| session.app.rect(id).x),
        [0.0, 240.0, 480.0]
    );
    let again = ok(
        &mut session,
        "/groups/reorder-child",
        json!({ "groupId": id, "childId": "c", "toIndex": 0 }),
    );
    assert_eq!(again, json!({ "changed": false }));
    // Two steps: the reorder, then the group and its layout.
    session.app.undo().undo();
    assert!(!session.app.app().can_undo());
    assert_eq!(session.app.rect("c").x, 600.0);

    for (body, status) in [
        (json!({}), 400),
        (json!({ "groupId": "a" }), 404),
        (json!({ "entityIds": ["a"] }), 404),
    ] {
        let refused = session.post("/groups/auto-layout", body.clone());
        assert_eq!(refused.status, status, "{body}: {}", refused.body);
    }
    let refused = session.post("/groups/reorder-child", json!({ "groupId": "g" }));
    assert_eq!(refused.status, 400, "{}", refused.body);

    let mut fresh = three_notes();
    let plain = ok(
        &mut fresh,
        "/groups/auto-layout",
        json!({ "entityIds": ["a", "b"] }),
    );
    assert_eq!(plain["label"], "Auto-layout");
}
