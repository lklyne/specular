//! `POST /tasks/apply`: the `breakpoints` verb.

use serde_json::{Value, json};
use specular_doc::{EntityId, Kind, VIEWPORT_PRESETS};

use crate::common::Scripted;

const URL: &str = "https://example.com/shop";

#[test]
fn breakpoints_lay_one_page_per_preset_in_a_row_inside_a_group_as_one_undo_step() {
    let mut session = Scripted::empty();
    let response = session.post(
        "/tasks/apply",
        json!({ "taskKind": "breakpoint_map", "input": { "url": URL } }),
    );
    assert_eq!(response.status, 200, "{}", response.body);
    let reply = response.body;

    assert_eq!(
        reply["resolvedPresets"],
        json!(["iPhone 14 Pro", "iPad Mini", "Desktop"])
    );
    assert_eq!(reply["taskKind"], "breakpoint_map");
    assert_eq!(reply["warnings"], json!([]));
    assert_eq!(reply["edgeIds"].as_array().map(Vec::len), Some(2));
    let group = reply["groupId"].as_str().unwrap_or_default();
    let pages: Vec<&str> = (reply["pageIds"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(pages.len(), 3);

    let document = session.app.document();
    let label = document
        .entity(&EntityId::from(group))
        .and_then(|e| e.label.clone());
    assert_eq!(label.as_deref(), Some("Breakpoints: example.com/shop"));
    let mut sync_ids = Vec::new();
    let mut next_x = reply["placement"]["canvasX"].as_f64().unwrap_or_default();
    for (id, name) in pages.iter().zip(["iPhone 14 Pro", "iPad Mini", "Desktop"]) {
        let preset = VIEWPORT_PRESETS
            .iter()
            .find(|p| p.label == name)
            .expect("preset");
        let entity = document.entity(&EntityId::from(*id)).expect("page exists");
        assert_eq!(entity.parent.as_ref().map(EntityId::as_str), Some(group));
        assert_eq!(
            (
                entity.rect.x,
                entity.rect.y,
                entity.rect.width,
                entity.rect.height
            ),
            (
                next_x,
                reply["placement"]["canvasY"].as_f64().unwrap_or_default(),
                preset.width,
                preset.height
            )
        );
        next_x += preset.width + 80.0;
        let Kind::Page(page) = &entity.kind else {
            panic!("{id} is not a page");
        };
        assert_eq!(page.url, URL);
        sync_ids.push(page.sync_id.clone());
    }
    assert!(
        sync_ids[0]
            .as_deref()
            .is_some_and(|id| id.starts_with("sync_"))
    );
    assert!(sync_ids.iter().all(|id| *id == sync_ids[0]));

    session.undo();
    session.app.assert_undo_returns_to_start();
    assert_eq!(session.canvas()["nodes"], json!([]));
    assert_eq!(session.canvas()["edges"], json!([]));
}

#[test]
fn breakpoints_refuse_what_electron_refuses() {
    let cases = [
        (
            json!({ "taskKind": "component_states", "input": { "url": URL } }),
            "Unsupported task kind: component_states",
        ),
        (
            json!({ "taskKind": "breakpoint_map", "input": { "url": URL, "presets": ["Nokia 3310"] } }),
            "Unknown preset label: Nokia 3310",
        ),
        (
            json!({ "taskKind": "breakpoint_map", "input": { "url": URL, "presets": ["Desktop", "Desktop"] } }),
            "Duplicate preset labels are not allowed",
        ),
    ];
    for (body, error) in cases {
        let mut session = Scripted::empty();
        let response = session.post("/tasks/apply", body);
        assert_eq!(response.status, 400);
        assert_eq!(response.body, json!({ "error": error }));
        assert_eq!(session.canvas()["nodes"], json!([]));
    }
}
