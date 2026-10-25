//! The verbs end to end: a request through the planned event and `update`,
//! then the response and the document.

use serde_json::{Value, json};
use specular_doc::Rect;
use specular_interact::{Action, Showing};
use specular_testkit::{TestApp, connected, document, sticky};

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

/// Mutation: dropping the `has_a_tab` arm from `act::focus` frames the page
/// on the canvas and shows no tab.
#[test]
fn focus_on_one_page_shows_its_tab_and_on_two_frames_them() {
    let mut session = Scripted::new(TestApp::with_pages(2));
    session.app.viewport((1000.0, 800.0));
    let focus = |session: &mut Scripted, ids: Value| {
        ok(session, "/camera/focus", json!({ "pageIds": ids }))
    };
    assert_eq!(
        focus(&mut session, json!(["p2"])),
        json!({ "focused": true })
    );
    assert_eq!(session.app.app().showing(), Showing::Item("p2".into()));
    assert_eq!(session.app.selected_ids(), ["p2"]);

    // Two pages have no tab between them: back on the canvas they are framed.
    session.app.act(Action::Show(Showing::Canvas));
    focus(&mut session, json!(["p1", "p2"]));
    assert_eq!(session.app.app().showing(), Showing::Canvas);
    assert_eq!(session.app.selected_ids(), ["p1"]);
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
