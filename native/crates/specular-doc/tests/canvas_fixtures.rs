//! Load, mutate and save round trips against `.canvas` files written by the
//! Electron app.
//!
//! - `rich-workspace.canvas` is a copy of the integration-test snapshot in
//!   `tests/integration/__snapshots__/` (five entity kinds, an edge, the
//!   top-level `specular` and `appState` extensions, nested drawing strokes).
//! - `pages.canvas` holds the four `link` nodes of
//!   `resources/starter-space/Welcome.canvas` verbatim (with their
//!   `metadata`, `syncId`, `presetIndex` extensions), that file's `appState`,
//!   and one hand-added edge between two pages. It has no
//!   `specular.entityOrder`, which a save adds.
#![expect(
    clippy::unwrap_used,
    reason = "shared test helpers panic to fail the test on a malformed fixture"
)]

use serde_json::{Value, json};
use specular_doc::{Command, Document, Entity, EntityId, History, Kind, Page, Rect};

const RICH_WORKSPACE: &str = include_str!("fixtures/rich-workspace.canvas");
const PAGES: &str = include_str!("fixtures/pages.canvas");

const SPECULAR_LAPTOP: &str = "page_7111e67c-a370-411c-a0a1-75313d1997d8";
const SPECULAR_PHONE: &str = "page_b9ce92c1-c91f-44fe-be29-b335332f67aa";

fn load(json: &str) -> Document {
    Document::from_canvas_str(json).unwrap()
}

fn parse(json: &str) -> Value {
    serde_json::from_str(json).unwrap()
}

fn saved(document: &Document) -> Value {
    document.to_canvas_value().unwrap()
}

/// `pages.canvas` as a save writes it: the fixture plus the stack order.
fn pages_saved() -> Value {
    let mut canvas = parse(PAGES);
    let ids = canvas["nodes"].as_array().unwrap().iter();
    let ids = ids.chain(canvas["edges"].as_array().unwrap());
    let order: Vec<Value> = ids.map(|item| item["id"].clone()).collect();
    canvas["specular"] = json!({ "entityOrder": order });
    canvas
}

fn node<'a>(canvas: &'a Value, id: &str) -> &'a Value {
    let nodes = canvas["nodes"].as_array().unwrap();
    nodes.iter().find(|node| node["id"] == id).unwrap()
}

fn set_rect(id: &str, rect: Rect) -> Command {
    Command::SetRect {
        id: id.into(),
        rect,
    }
}

/// Removing an entity takes its edges with it, as one undo step.
fn remove_with_edges(document: &Document, id: &str) -> Command {
    let id = EntityId::from(id);
    let edges = document.edges_touching(&id);
    let mut commands: Vec<_> = edges
        .map(|edge| Command::RemoveEdge(edge.id.clone()))
        .collect();
    commands.push(Command::RemoveEntity(id));
    Command::Batch(commands)
}

#[test]
fn rich_workspace_round_trips_to_same_json_value() {
    assert_eq!(saved(&load(RICH_WORKSPACE)), parse(RICH_WORKSPACE));

    {
        assert_eq!(saved(&load(PAGES)), pages_saved());
    }
}

#[test]
fn saving_a_reloaded_document_is_byte_stable() {
    let first = load(PAGES).to_canvas_string().unwrap();
    let second = load(&first).to_canvas_string().unwrap();
    assert_eq!(second, first);
}

#[test]
fn pages_fixture_lists_every_link_node() {
    let document = load(PAGES);
    let urls: Vec<_> = document
        .entities()
        .filter_map(|entity| match &entity.kind {
            Kind::Page(page) => Some(page.url.as_str()),
            Kind::Text(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) | Kind::Shape(_) => {
                None
            }
        })
        .collect();
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
fn rich_workspace_has_every_kind_but_page() {
    let document = load(RICH_WORKSPACE);
    let kinds: Vec<_> = document
        .entities()
        .map(|entity| entity.kind.name())
        .collect();
    assert_eq!(kinds, ["text", "file", "drawing", "shape", "group"]);
    assert_eq!(document.children(&"generated-id-5".into()).count(), 2);
}

#[test]
fn moving_a_page_preserves_its_unknown_fields() {
    let mut document = load(PAGES);
    document
        .apply(set_rect(
            SPECULAR_LAPTOP,
            Rect::new(0.0, 0.0, 1280.0, 800.0),
        ))
        .unwrap();
    let mut expected = node(&parse(PAGES), SPECULAR_LAPTOP).clone();
    expected["x"] = json!(0);
    expected["y"] = json!(0);
    assert_eq!(node(&saved(&document), SPECULAR_LAPTOP), &expected);
}

#[test]
fn mutations_leave_top_level_extensions_untouched() {
    let mut document = load(RICH_WORKSPACE);
    let remove = remove_with_edges(&document, "generated-id-1");
    document.apply(remove).unwrap();
    let saved = saved(&document);
    let original = parse(RICH_WORKSPACE);
    assert_eq!(saved["appState"], original["appState"]);
    assert_eq!(saved["edges"], original["edges"]);
}

#[test]
fn undoing_move_restores_fixture_exactly() {
    let mut document = load(PAGES);
    let mut history = History::new();
    let moved = set_rect(SPECULAR_PHONE, Rect::new(12.5, -40.0, 430.0, 932.0));
    history.apply(&mut document, moved).unwrap();
    assert!(history.undo(&mut document).unwrap().is_some());
    assert_eq!(saved(&document), pages_saved());

    {
        let mut document = load(PAGES);
        let mut history = History::new();
        let page = Kind::Page(Page {
            url: "http://localhost:4321/garden".to_owned(),
            ..Page::default()
        });
        let entity = Entity::new("page_new", Rect::new(5000.0, 360.0, 1440.0, 900.0), page);
        let add = Command::InsertEntity {
            entity: Box::new(entity),
            at: document.stack_len(),
        };
        history.apply(&mut document, add).unwrap();
        assert_eq!(saved(&document)["nodes"].as_array().unwrap().len(), 5);
        assert!(history.undo(&mut document).unwrap().is_some());
        assert_eq!(saved(&document), pages_saved());
    }

    {
        let mut document = load(PAGES);
        let mut history = History::new();
        let remove = remove_with_edges(&document, SPECULAR_PHONE);
        history.apply(&mut document, remove).unwrap();
        assert!(history.undo(&mut document).unwrap().is_some());
        assert_eq!(saved(&document), pages_saved());
    }
}

#[test]
fn removing_a_page_drops_its_edge() {
    let mut document = load(PAGES);
    let remove = remove_with_edges(&document, SPECULAR_LAPTOP);
    document.apply(remove).unwrap();
    assert_eq!(saved(&document)["edges"], json!([]));
}

#[test]
fn undo_then_redo_of_a_mutation_sequence_replays_it() {
    let mut document = load(PAGES);
    let mut history = History::new();
    let moved = set_rect(SPECULAR_PHONE, Rect::new(0.0, 0.0, 393.0, 852.0));
    history.apply(&mut document, moved).unwrap();
    let remove = remove_with_edges(&document, SPECULAR_LAPTOP);
    history.apply(&mut document, remove).unwrap();
    let after = saved(&document);
    while history.undo(&mut document).unwrap().is_some() {}
    assert_eq!(saved(&document), pages_saved());
    while history.redo(&mut document).unwrap().is_some() {}
    assert_eq!(saved(&document), after);
}
