use serde_json::{Value, json};

use super::CanvasError;
use crate::{
    Color, ColorPreset, Command, Document, EdgeId, EdgeKind, EntityId, ItemId, JsonMap, Kind,
    LayoutMode, TextStyle,
};

fn load(value: &Value) -> Document {
    Document::from_canvas_value(value.clone()).unwrap()
}

fn saved(document: &Document) -> Value {
    document.to_canvas_value().unwrap()
}

fn canvas(nodes: Value, edges: Value) -> Value {
    let fields = [("nodes".to_owned(), nodes), ("edges".to_owned(), edges)];
    Value::Object(fields.into_iter().collect())
}

fn sorted_keys(map: &JsonMap) -> Vec<&str> {
    let mut keys: Vec<_> = map.keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys
}

fn entity_ids(document: &Document) -> Vec<&str> {
    document.entities().map(|e| e.id.as_str()).collect()
}

/// One node of every kind with every modeled field set, plus edges,
/// annotations and fields from another tool at each level.
fn everything() -> Value {
    let anchor = json!({
        "pageId": "page", "pageUrl": "https://example.com/", "scrollX": 0, "scrollY": 120.5,
        "element": { "selector": "#cta", "docX": 10, "docY": 20, "viewportPositioned": true },
    });
    json!({
        "nodes": [
            { "id": "page", "type": "link", "x": 0, "y": 0, "width": 1280, "height": 800,
              "url": "https://example.com/", "presetIndex": 2, "syncId": "sync-1",
              "label": "Home", "source": "generated", "groupId": "group",
              "parentGroupId": "group", "metadata": { "pageSizeMode": "preset" },
              "colorScheme": "dark", "otherTool": { "keep": [1, 2.5] } },
            { "id": "text", "type": "text", "x": 10.25, "y": -4, "width": 200, "height": 100,
              "text": "hello", "color": "1",
              "specular": { "textStyle": "plain", "widthMode": "auto", "colorRole": "neutral",
                "textSize": 18, "textFont": "mono", "pageAnchor": anchor,
                "parentGroupId": "group", "label": "Note", "otherTool": true } },
            { "id": "file", "type": "file", "x": 0, "y": 0, "width": 320, "height": 480,
              "file": "notes/a.md", "subpath": "#intro", "objectFit": "cover",
              "presetIndex": 1, "metadata": { "k": "v" },
              "specular": { "parentGroupId": "group", "label": "Doc" } },
            { "id": "drawing", "type": "drawing", "x": 0, "y": 0, "width": 50, "height": 50,
              "strokes": [{ "id": "s1", "color": "#ff0000", "width": 2,
                "points": [{ "x": 0, "y": 0 }, { "x": 10.5, "y": 10 }],
                "brushType": "highlight", "pressure": [0.5] }],
              "label": "Sketch", "parentGroupId": "group", "pageAnchor": anchor },
            { "id": "shape", "type": "shape", "x": 0, "y": 0, "width": 120, "height": 80,
              "shapeKind": "diamond", "text": "box", "color": "4", "strokeWidth": 2,
              "borderStyle": "dashed", "borderColor": "neutral", "theme": "blueprint",
              "label": "Box", "parentGroupId": "group", "pageAnchor": anchor,
              "specular": { "textSize": 14, "fillStyle": "none", "textAlign": "left",
                "textVerticalAlign": "top" } },
            { "id": "group", "type": "group", "x": -24, "y": -24, "width": 2000, "height": 900,
              "label": "All", "color": "#00ff00", "layoutMode": "row", "layoutGap": 16,
              "parentGroupId": "outer", "managedLayout": true, "groupColor": "#00ff00",
              "sourceTaskId": "task-1", "groupMetadata": { "k": 1 },
              "background": "bg.png" },
        ],
        "edges": [
            { "id": "edge", "fromNode": "page", "toNode": "shape", "fromSide": "right",
              "toSide": "left", "fromEnd": "none", "toEnd": "arrow", "color": "5",
              "label": "to", "strokeWidth": 2, "lineStyle": "dashed",
              "edgeKind": "connection", "edgeMetadata": { "k": 1 }, "otherTool": 1 },
        ],
        "specular": {
            "entityOrder": ["page", "edge", "text", "file", "drawing", "shape", "group"],
            "otherTool": "kept",
        },
        "annotations": [
            { "id": "a1", "anchor": { "type": "page", "pageId": "page", "offsetX": 3, "offsetY": 4 },
              "author": "user", "text": "fix", "status": "pending",
              "replies": [{ "author": "agent", "text": "done", "timestamp": "2026-01-01T00:01:00.000Z" }],
              "createdAt": "2026-01-01T00:00:00.000Z", "elementName": "CTA",
              "pageAnchor": anchor, "metadata": { "pageName": "Home" }, "otherTool": true },
            { "id": "a2", "anchor": { "type": "region",
                "docRect": { "x": 1, "y": 2, "width": 3, "height": 4 } },
              "author": "agent", "text": "", "status": "resolved", "replies": [],
              "createdAt": "2026-01-02T00:00:00.000Z" },
        ],
        "appState": { "zoom": 0.411_999_999_999_999_53, "pan": { "x": -421, "y": 109 } },
        "otherTool": { "version": 3 },
    })
}

#[test]
fn every_modeled_field_round_trips() {
    let file = everything();
    assert_eq!(saved(&load(&file)), file);
}

#[test]
fn modeled_fields_are_read_into_typed_fields_not_extra() {
    let document = load(&everything());
    let group = EntityId::from("group");
    for entity in document.entities() {
        let foreign = match entity.id.as_str() {
            "page" => vec!["otherTool"],
            "text" => vec!["specular"],
            "group" => vec!["background"],
            _ => vec![],
        };
        assert_eq!(
            entity.extra.keys().collect::<Vec<_>>(),
            foreign,
            "{}",
            entity.id
        );
        assert!(entity.label.is_some(), "{}", entity.id);
        let parent = if entity.id == group { "outer" } else { "group" };
        assert_eq!(entity.parent, Some(EntityId::from(parent)), "{}", entity.id);
    }
    assert_eq!(document.children(&group).count(), 5);

    let text = document.entity(&"text".into()).unwrap();
    assert_eq!(text.extra["specular"], json!({ "otherTool": true }));
    assert_eq!(text.anchor.as_ref().unwrap().scroll_y, Some(120.5));
    let Kind::Text(fields) = &text.kind else {
        panic!("not a text");
    };
    assert_eq!(fields.color, Some(Color::Neutral));
    assert_eq!(fields.style, Some(TextStyle::Plain));

    let Kind::Group(fields) = &document.entity(&group).unwrap().kind else {
        panic!("not a group");
    };
    assert_eq!(fields.layout_mode, Some(LayoutMode::Row));
    assert_eq!(fields.color, Some(Color::Custom("#00ff00".to_owned())));

    let edge = document.edge(&"edge".into()).unwrap();
    assert_eq!(edge.kind, Some(EdgeKind::Connection));
    assert_eq!(edge.extra.keys().collect::<Vec<_>>(), ["otherTool"]);
    assert_eq!(document.annotations().len(), 2);
    assert_eq!(
        sorted_keys(document.extra()),
        ["appState", "otherTool", "specular"]
    );
}

#[test]
fn stack_order_follows_entity_order_then_file_order() {
    let file = json!({
        "nodes": [
            { "id": "a", "type": "group", "x": 0, "y": 0, "width": 1, "height": 1 },
            { "id": "b", "type": "group", "x": 0, "y": 0, "width": 1, "height": 1 },
            { "id": "c", "type": "group", "x": 0, "y": 0, "width": 1, "height": 1 },
        ],
        "edges": [{ "id": "e", "fromNode": "a", "toNode": "b" }],
        "specular": { "entityOrder": ["c", "gone", "e", "c", "a"] },
    });
    let document = load(&file);
    let order: Vec<_> = document.order().iter().map(ItemId::as_str).collect();
    assert_eq!(order, ["c", "e", "a", "b"]);
    assert!(matches!(document.order()[1], ItemId::Edge(_)));

    let saved = saved(&document);
    assert_eq!(
        saved["specular"]["entityOrder"],
        json!(["c", "e", "a", "b"])
    );
    let node_ids: Vec<_> = saved["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| &n["id"])
        .collect();
    assert_eq!(node_ids, ["c", "a", "b"]);
}

#[test]
fn values_that_do_not_fit_their_field_stay_in_extra() {
    let file = canvas(
        json!([
            { "id": "p", "type": "link", "x": 0, "y": 0, "width": 1, "height": 1,
              "url": "https://example.com/", "syncId": null, "source": "imported",
              "presetIndex": "big" },
            { "id": "t", "type": "text", "x": 0, "y": 0, "width": 1, "height": 1, "text": "",
              "specular": { "textStyle": "banner", "pageAnchor": { "pageUrl": "no page id" } } },
        ]),
        json!([{ "id": "e", "fromNode": "p", "toNode": "t", "edgeKind": "depends_on",
                 "label": null, "toSide": "left" }]),
    );
    let mut document = load(&file);
    let page = document.entity(&"p".into()).unwrap();
    assert_eq!(
        sorted_keys(&page.extra),
        ["presetIndex", "source", "syncId"]
    );
    assert!(document.entity(&"t".into()).unwrap().anchor.is_none());
    let edge = document.edge(&"e".into()).unwrap().clone();
    assert_eq!((edge.kind, edge.to_side.is_some()), (None, true));
    assert_eq!(strip_order(saved(&document)), file);

    // Once the app sets the field, the typed value replaces the leftover.
    let mut edge = edge;
    edge.kind = Some(EdgeKind::Connection);
    document
        .apply(Command::ReplaceEdge(Box::new(edge)))
        .unwrap();
    assert_eq!(saved(&document)["edges"][0]["edgeKind"], "connection");
}

#[test]
fn items_that_cannot_be_typed_are_kept_raw_and_written_back() {
    let good = json!({ "id": "a", "type": "group", "x": 0, "y": 0, "width": 1, "height": 1 });
    let strays = json!([
        { "id": "widget", "type": "embed", "x": 0, "y": 0, "width": 1, "height": 1 },
        { "id": "star", "type": "shape", "x": 0, "y": 0, "width": 1, "height": 1,
          "shapeKind": "star", "text": "kept" },
        { "id": "no-rect", "type": "text", "text": "kept" },
        { "id": "a", "type": "text", "x": 5, "y": 5, "width": 1, "height": 1, "text": "dup" },
        "not an object",
    ]);
    let stray_edges = json!([{ "id": "a", "fromNode": "a", "toNode": "a" }, { "id": "e2" }]);
    let stray_annotation = json!({ "id": "n1", "text": "no anchor" });

    let mut nodes = vec![good.clone()];
    nodes.extend(strays.as_array().unwrap().iter().cloned());
    let file = json!({
        "nodes": nodes, "edges": stray_edges, "annotations": [stray_annotation],
    });
    let document = load(&file);
    assert_eq!(entity_ids(&document), ["a"]);
    assert_eq!(document.edges().count(), 0);
    assert_eq!(document.annotations(), []);
    assert_eq!(document.extra()["nodes"], strays);

    let first = saved(&document);
    assert_eq!(strip_order(first.clone()), file);
    assert_eq!(saved(&load(&first)), first);
}

#[test]
fn neutral_is_stored_as_the_red_preset_plus_a_color_role() {
    let mut document = load(&canvas(
        json!([{ "id": "s", "type": "shape", "x": 0, "y": 0, "width": 1, "height": 1,
                 "shapeKind": "pill", "color": "1" }]),
        json!([]),
    ));
    let id = EntityId::from("s");
    let Kind::Shape(mut shape) = document.entity(&id).unwrap().kind.clone() else {
        panic!("not a shape");
    };
    assert_eq!(shape.color, Some(Color::Preset(ColorPreset::Red)));
    shape.color = Some(Color::Neutral);
    let kind = Box::new(Kind::Shape(shape));
    document
        .apply(Command::SetKind {
            id: id.clone(),
            kind,
        })
        .unwrap();

    let node = saved(&document)["nodes"][0].clone();
    assert_eq!(node["color"], "1");
    assert_eq!(node["specular"], json!({ "colorRole": "neutral" }));
    let reloaded = load(&saved(&document));
    let Kind::Shape(shape) = &reloaded.entity(&id).unwrap().kind else {
        panic!("not a shape");
    };
    assert_eq!(shape.color, Some(Color::Neutral));
}

#[test]
fn group_membership_copies_are_dropped_and_group_color_wins() {
    let document = load(&canvas(
        json!([{ "id": "g", "type": "group", "x": 0, "y": 0, "width": 1, "height": 1,
                 "color": "2", "groupColor": "neutral", "pageIds": ["p"], "entityIds": ["p"] }]),
        json!([]),
    ));
    let node = saved(&document)["nodes"][0].clone();
    assert_eq!(
        node,
        json!({ "id": "g", "type": "group", "x": 0, "y": 0, "width": 1, "height": 1,
                "color": "neutral", "groupColor": "neutral" })
    );
}

#[test]
fn page_reads_the_older_group_id_and_writes_both_names() {
    let document = load(&canvas(
        json!([{ "id": "p", "type": "link", "x": 0, "y": 0, "width": 1, "height": 1,
                 "url": "https://example.com/", "groupId": "g" }]),
        json!([]),
    ));
    assert_eq!(
        document.entity(&"p".into()).unwrap().parent,
        Some("g".into())
    );
    let node = saved(&document)["nodes"][0].clone();
    assert_eq!(
        (&node["groupId"], &node["parentGroupId"]),
        (&json!("g"), &json!("g"))
    );
}

#[test]
fn numbers_are_rounded_to_hundredths_except_zoom() {
    let mut document = load(&json!({
        "nodes": [{ "id": "g", "type": "group", "x": 210.954_545_454_545_47, "y": -0.125,
                    "width": 100.0, "height": -0.001 }],
        "edges": [],
        "appState": { "zoom": 0.123_456_789, "pan": { "x": 1.005_1, "y": 2.0 } },
    }));
    document.extra_mut().insert("zoom".to_owned(), json!(3.0));
    let text = document.to_canvas_string().unwrap();
    let saved: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        saved["nodes"][0],
        json!({ "id": "g", "type": "group", "x": 210.95, "y": -0.12, "width": 100, "height": 0 })
    );
    assert_eq!(
        saved["appState"],
        json!({ "zoom": 0.123_456_789, "pan": { "x": 1.01, "y": 2 } })
    );
    assert!(
        text.contains("\"width\": 100,") && text.contains("\"zoom\": 3\n"),
        "{text}"
    );
}

#[test]
fn an_empty_document_writes_the_two_spec_arrays() {
    assert_eq!(saved(&Document::new()), json!({ "nodes": [], "edges": [] }));
    assert_eq!(
        saved(&load(&json!({}))),
        json!({ "nodes": [], "edges": [] })
    );
}

type ErrorCheck = fn(&CanvasError) -> bool;

#[test]
fn malformed_files_are_refused() {
    let rows: [(&str, &str, ErrorCheck); 3] = [
        ("not json", "{", |e| matches!(e, CanvasError::Parse(_))),
        ("not an object", "[]", |e| {
            matches!(e, CanvasError::NotAnObject)
        }),
        ("nodes not an array", r#"{"nodes": {}}"#, |e| {
            matches!(e, CanvasError::NotAnArray("nodes"))
        }),
    ];
    for (name, text, is_expected) in rows {
        let error = Document::from_canvas_str(text).unwrap_err();
        assert!(is_expected(&error), "{name}: {error:?}");
    }
}

#[test]
fn removed_edge_leaves_the_saved_order() {
    let mut document = load(&everything());
    document
        .apply(Command::RemoveEdge(EdgeId::from("edge")))
        .unwrap();
    let saved = saved(&document);
    assert_eq!(saved["edges"], json!([]));
    assert_eq!(
        saved["specular"]["entityOrder"].as_array().unwrap().len(),
        6
    );
    assert_eq!(saved["specular"]["otherTool"], "kept");
}

/// Drops the `specular.entityOrder` a save always adds, for comparing with
/// a file that had none.
fn strip_order(mut canvas: Value) -> Value {
    canvas.as_object_mut().unwrap().remove("specular");
    canvas
}
