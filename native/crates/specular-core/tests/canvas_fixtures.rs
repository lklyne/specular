//! Load/mutate/save round trips against `.canvas` files written by the
//! Electron app.
//!
//! - `rich-workspace.canvas` is a copy of the integration-test snapshot in
//!   `tests/integration/__snapshots__/` (every entity kind, top-level
//!   `specular` and `appState` extensions, nested drawing strokes).
//! - `pages.canvas` holds the four `link` nodes of
//!   `resources/starter-space/Welcome.canvas` verbatim (with their
//!   `metadata`, `syncId`, `presetIndex` extensions), that file's `appState`,
//!   and one hand-added edge between two pages.
#![expect(
    clippy::unwrap_used,
    reason = "shared test helpers panic to fail the test on a malformed fixture"
)]

use serde_json::{Value, json};
use specular_core::document::PageNode;
use specular_core::{CanvasDocument, CanvasRect};

const RICH_WORKSPACE: &str = include_str!("fixtures/rich-workspace.canvas");
const PAGES: &str = include_str!("fixtures/pages.canvas");

const SPECULAR_LAPTOP: &str = "page_7111e67c-a370-411c-a0a1-75313d1997d8";
const SPECULAR_PHONE: &str = "page_b9ce92c1-c91f-44fe-be29-b335332f67aa";

fn load(json: &str) -> CanvasDocument {
    CanvasDocument::from_json_canvas(json).unwrap()
}

fn parse(json: &str) -> Value {
    serde_json::from_str(json).unwrap()
}

fn saved(document: &CanvasDocument) -> Value {
    parse(&document.to_json_canvas().unwrap())
}

fn node<'a>(canvas: &'a Value, id: &str) -> &'a Value {
    canvas["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == id)
        .unwrap()
}

#[test]
fn rich_workspace_round_trips_to_same_json_value() {
    assert_eq!(saved(&load(RICH_WORKSPACE)), parse(RICH_WORKSPACE));
}

#[test]
fn pages_fixture_round_trips_to_same_json_value() {
    assert_eq!(saved(&load(PAGES)), parse(PAGES));
}

#[test]
fn saving_a_reloaded_document_is_byte_stable() {
    let first = load(PAGES).to_json_canvas().unwrap();
    let second = load(&first).to_json_canvas().unwrap();
    assert_eq!(second, first);
}

#[test]
fn pages_fixture_lists_every_link_node() {
    let urls: Vec<_> = load(PAGES).pages().into_iter().map(|p| p.url).collect();
    assert_eq!(
        urls,
        [
            "https://cursoreffects.com/",
            "https://specular.sh/",
            "https://specular.sh/",
            "https://specular.sh/",
        ]
    );
}

#[test]
fn rich_workspace_has_no_pages() {
    assert!(load(RICH_WORKSPACE).pages().is_empty());
}

#[test]
fn moving_a_page_preserves_its_unknown_fields() {
    let mut document = load(PAGES);
    document
        .set_node_rect(SPECULAR_LAPTOP, CanvasRect::new(0.0, 0.0, 1280.0, 800.0))
        .unwrap();
    let mut expected = node(&parse(PAGES), SPECULAR_LAPTOP).clone();
    expected["x"] = json!(0);
    expected["y"] = json!(0);
    assert_eq!(node(&saved(&document), SPECULAR_LAPTOP), &expected);
}

#[test]
fn mutations_leave_top_level_extensions_untouched() {
    let mut document = load(RICH_WORKSPACE);
    document.remove_node("generated-id-2").unwrap();
    let saved = saved(&document);
    let original = parse(RICH_WORKSPACE);
    assert_eq!(
        (&saved["appState"], &saved["specular"]),
        (&original["appState"], &original["specular"])
    );
}

#[test]
fn undoing_move_restores_fixture_exactly() {
    let mut document = load(PAGES);
    document
        .set_node_rect(SPECULAR_PHONE, CanvasRect::new(12.5, -40.0, 430.0, 932.0))
        .unwrap();
    assert!(document.undo());
    assert_eq!(saved(&document), parse(PAGES));
}

#[test]
fn undoing_add_restores_fixture_exactly() {
    let mut document = load(PAGES);
    let page = PageNode {
        node_id: "page_new".to_owned(),
        url: "http://localhost:4321/garden".to_owned(),
        rect: CanvasRect::new(5000.0, 360.0, 1440.0, 900.0),
    };
    document.add_page(&page).unwrap();
    assert!(document.undo());
    assert_eq!(saved(&document), parse(PAGES));
}

#[test]
fn undoing_remove_restores_fixture_exactly() {
    let mut document = load(PAGES);
    document.remove_node(SPECULAR_PHONE).unwrap();
    assert!(document.undo());
    assert_eq!(saved(&document), parse(PAGES));
}

#[test]
fn removing_a_page_drops_its_edge() {
    let mut document = load(PAGES);
    document.remove_node(SPECULAR_LAPTOP).unwrap();
    assert_eq!(saved(&document)["edges"], json!([]));
}

#[test]
fn undo_then_redo_of_a_mutation_sequence_replays_it() {
    let mut document = load(PAGES);
    document
        .set_node_rect(SPECULAR_PHONE, CanvasRect::new(0.0, 0.0, 393.0, 852.0))
        .unwrap();
    document.remove_node(SPECULAR_LAPTOP).unwrap();
    let after = saved(&document);
    while document.undo() {}
    while document.redo() {}
    assert_eq!(saved(&document), after);
}
