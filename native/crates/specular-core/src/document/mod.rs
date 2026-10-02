//! The canvas document: JSON Canvas content held in a yrs (Yjs) doc.
//!
//! Mirrors the Electron app's two-layer model in miniature: the yrs doc is the
//! persisted, undoable truth; every mutation is one transaction and one undo
//! step; `.canvas` JSON is a projection of it.
//!
//! Layout of the yrs doc:
//! - `nodes`: root map, node id -> nested map of that node's JSON fields, so
//!   moving a page touches only its `x`/`y` keys.
//! - `nodeOrder`: root array of node ids, back-to-front.
//! - `edges`: root array, one JSON object per edge.
//! - `canvas`: root map of unknown top-level fields (`appState`, `specular`,
//!   `annotations`, ...). Kept outside the undo scope, matching the app,
//!   where viewport and UI state are not undoable.

mod any_json;

use std::collections::HashSet;

use serde_json::Value;
use yrs::types::ToJson;
use yrs::{
    Any, Array, ArrayRef, Doc, In, Map, MapPrelim, MapRef, Out, ReadTxn, Transact, TransactionMut,
    UndoManager,
};

use self::any_json::{any_map_to_object, any_to_value, object_to_any_map, value_to_any};
use crate::geometry::CanvasRect;
use crate::json_canvas::{Edge, JsonCanvas, Node, whole_number};

const NODES: &str = "nodes";
const NODE_ORDER: &str = "nodeOrder";
const EDGES: &str = "edges";
const CANVAS_EXTRA: &str = "canvas";

/// Errors loading, saving, or mutating a [`CanvasDocument`].
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// The input is not valid JSON Canvas, or a stored node no longer is.
    #[error("invalid JSON Canvas: {0}")]
    Parse(#[from] serde_json::Error),
    /// No node with this id exists.
    #[error("unknown node {0:?}")]
    UnknownNode(String),
    /// A node with this id already exists.
    #[error("duplicate node id {0:?}")]
    DuplicateNode(String),
}

/// A page as the document stores it (a `link` node).
#[derive(Debug, Clone, PartialEq)]
pub struct PageNode {
    /// The node id.
    pub node_id: String,
    /// Full URL.
    pub url: String,
    /// Placement in canvas space.
    pub rect: CanvasRect,
}

/// A yrs-backed canvas document with one undo step per mutation.
pub struct CanvasDocument {
    doc: Doc,
    nodes: MapRef,
    node_order: ArrayRef,
    edges: ArrayRef,
    extra: MapRef,
    undo: UndoManager,
}

impl std::fmt::Debug for CanvasDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CanvasDocument").finish_non_exhaustive()
    }
}

impl Default for CanvasDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasDocument {
    /// An empty document.
    pub fn new() -> Self {
        let doc = Doc::new();
        let nodes = doc.get_or_insert_map(NODES);
        let node_order = doc.get_or_insert_array(NODE_ORDER);
        let edges = doc.get_or_insert_array(EDGES);
        let extra = doc.get_or_insert_map(CANVAS_EXTRA);
        let mut undo = UndoManager::new();
        undo.expand_scope(&doc, &nodes);
        undo.expand_scope(&doc, &node_order);
        undo.expand_scope(&doc, &edges);
        Self {
            doc,
            nodes,
            node_order,
            edges,
            extra,
            undo,
        }
    }

    /// Loads a `.canvas` file's contents. Loading is not an undo step.
    pub fn from_json_canvas(json: &str) -> Result<Self, DocumentError> {
        let canvas: JsonCanvas = serde_json::from_str(json)?;
        let mut document = Self::new();
        {
            let mut txn = document.doc.transact_mut();
            let mut seen = HashSet::with_capacity(canvas.nodes.len());
            for node in &canvas.nodes {
                if !seen.insert(node.id.as_str()) {
                    return Err(DocumentError::DuplicateNode(node.id.clone()));
                }
                document.insert_node(&mut txn, &node.id, &serde_json::to_value(node)?);
            }
            for edge in &canvas.edges {
                let Value::Object(fields) = serde_json::to_value(edge)? else {
                    continue;
                };
                document
                    .edges
                    .push_back(&mut txn, Any::from(object_to_any_map(&fields)));
            }
            for (key, value) in &canvas.extra {
                document
                    .extra
                    .insert(&mut txn, key.as_str(), value_to_any(value));
            }
        }
        // Drop the load from history so the first undo is a user mutation.
        document.undo.clear_all();
        Ok(document)
    }

    /// Serializes the document back to JSON Canvas (pretty-printed, so the
    /// file stays diffable). Loading then saving an unmodified document yields
    /// the same JSON value; key order within objects may differ.
    pub fn to_json_canvas(&self) -> Result<String, DocumentError> {
        let txn = self.doc.transact();
        let mut canvas = JsonCanvas::default();
        for id in self.node_ids(&txn) {
            if let Some(node) = self.read_node(&txn, &id) {
                canvas.nodes.push(node?);
            }
        }
        for edge in self.edges.iter(&txn) {
            let Out::Any(Any::Map(fields)) = edge else {
                continue;
            };
            let value = Value::Object(any_map_to_object(&fields));
            canvas.edges.push(serde_json::from_value::<Edge>(value)?);
        }
        canvas.extra = self
            .extra
            .iter(&txn)
            .map(|(key, value)| (key.to_owned(), any_to_value(&value.to_json(&txn))))
            .collect();
        drop(txn);
        Ok(serde_json::to_string_pretty(&canvas)?)
    }

    /// Every page (`link` node), back-to-front. A node whose stored fields
    /// no longer parse is an error, as in [`to_json_canvas`](Self::to_json_canvas),
    /// rather than a page silently missing from the canvas.
    pub fn pages(&self) -> Result<Vec<PageNode>, DocumentError> {
        let txn = self.doc.transact();
        self.node_ids(&txn)
            .iter()
            .filter_map(|id| {
                let node = match self.read_node(&txn, id)? {
                    Ok(node) => node,
                    Err(error) => return Some(Err(error.into())),
                };
                let url = node.link_url()?.to_owned();
                Some(Ok(PageNode {
                    rect: CanvasRect::new(
                        node.x as f32,
                        node.y as f32,
                        node.width as f32,
                        node.height as f32,
                    ),
                    node_id: node.id,
                    url,
                }))
            })
            .collect()
    }

    /// Adds a page as a new front-most `link` node, in one transaction and
    /// one undo step.
    pub fn add_page(&mut self, page: &PageNode) -> Result<(), DocumentError> {
        {
            let mut txn = self.doc.transact_mut();
            if self.nodes.contains_key(&txn, &page.node_id) {
                return Err(DocumentError::DuplicateNode(page.node_id.clone()));
            }
            let fields = MapPrelim::from([
                ("id", In::Any(Any::from(page.node_id.as_str()))),
                ("type", In::Any(Any::from("link"))),
                ("x", In::Any(canvas_number(page.rect.x))),
                ("y", In::Any(canvas_number(page.rect.y))),
                ("width", In::Any(canvas_number(page.rect.width))),
                ("height", In::Any(canvas_number(page.rect.height))),
                ("url", In::Any(Any::from(page.url.as_str()))),
            ]);
            self.nodes.insert(&mut txn, page.node_id.as_str(), fields);
            self.node_order
                .push_back(&mut txn, Any::from(page.node_id.as_str()));
        }
        self.end_undo_step();
        Ok(())
    }

    /// Moves/resizes a node in one transaction and one undo step.
    pub fn set_node_rect(&mut self, node_id: &str, rect: CanvasRect) -> Result<(), DocumentError> {
        {
            let mut txn = self.doc.transact_mut();
            let Some(Out::YMap(node)) = self.nodes.get(&txn, node_id) else {
                return Err(DocumentError::UnknownNode(node_id.to_owned()));
            };
            node.insert(&mut txn, "x", canvas_number(rect.x));
            node.insert(&mut txn, "y", canvas_number(rect.y));
            node.insert(&mut txn, "width", canvas_number(rect.width));
            node.insert(&mut txn, "height", canvas_number(rect.height));
        }
        self.end_undo_step();
        Ok(())
    }

    /// Removes a node and every edge attached to it, in one transaction and
    /// one undo step (as deleting a canvas item does in the app).
    pub fn remove_node(&mut self, node_id: &str) -> Result<(), DocumentError> {
        {
            let mut txn = self.doc.transact_mut();
            if self.nodes.remove(&mut txn, node_id).is_none() {
                return Err(DocumentError::UnknownNode(node_id.to_owned()));
            }
            let order_index = self
                .node_order
                .iter(&txn)
                .position(|id| matches!(id, Out::Any(Any::String(id)) if &*id == node_id));
            if let Some(index) = order_index {
                self.node_order.remove(&mut txn, index as u32);
            }
            let attached: Vec<u32> = self
                .edges
                .iter(&txn)
                .enumerate()
                .filter(|(_, edge)| edge_touches(edge, node_id))
                .map(|(index, _)| index as u32)
                .collect();
            // Back to front, so earlier indices stay valid.
            for index in attached.into_iter().rev() {
                self.edges.remove(&mut txn, index);
            }
        }
        self.end_undo_step();
        Ok(())
    }

    /// Undoes the last mutation; `false` when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        self.undo.undo_blocking()
    }

    /// Redoes the last undone mutation; `false` when there is nothing to redo.
    pub fn redo(&mut self) -> bool {
        self.undo.redo_blocking()
    }

    fn insert_node(&self, txn: &mut TransactionMut<'_>, id: &str, node: &Value) {
        let fields: MapPrelim = match node {
            Value::Object(fields) => fields
                .iter()
                .map(|(key, field)| (key.as_str(), In::Any(value_to_any(field))))
                .collect(),
            _ => MapPrelim::default(),
        };
        self.nodes.insert(txn, id, fields);
        self.node_order.push_back(txn, Any::from(id));
    }

    fn node_ids<T: ReadTxn>(&self, txn: &T) -> Vec<String> {
        self.node_order
            .iter(txn)
            .filter_map(|id| match id {
                Out::Any(Any::String(id)) => Some(id.to_string()),
                _ => None,
            })
            .collect()
    }

    /// The node stored under `id`; `None` if it is missing, `Some(Err)` if its
    /// fields no longer form a valid JSON Canvas node.
    fn read_node<T: ReadTxn>(&self, txn: &T, id: &str) -> Option<Result<Node, serde_json::Error>> {
        let Some(Out::YMap(node)) = self.nodes.get(txn, id) else {
            return None;
        };
        Some(serde_json::from_value(any_to_value(&node.to_json(txn))))
    }

    /// Closes the undo capture window so rapid mutations stay separate steps.
    fn end_undo_step(&mut self) {
        self.undo.reset();
    }
}

fn edge_touches(edge: &Out, node_id: &str) -> bool {
    let Out::Any(Any::Map(fields)) = edge else {
        return false;
    };
    ["fromNode", "toNode"]
        .iter()
        .any(|key| matches!(fields.get(*key), Some(Any::String(id)) if &**id == node_id))
}

/// A coordinate as stored in the doc: whole numbers stay integral so saved
/// files read `100`, not `100.0`.
fn canvas_number(value: f32) -> Any {
    let value = f64::from(value);
    match whole_number(value) {
        Some(int) => Any::from(int),
        None => Any::from(value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"nodes":[
        {"id":"p1","type":"link","x":0,"y":0,"width":1280,"height":800,"url":"https://a.test/","presetIndex":1},
        {"id":"t1","type":"text","x":5,"y":5,"width":10,"height":10,"text":"note"},
        {"id":"p2","type":"link","x":1400,"y":0,"width":390,"height":844,"url":"https://b.test/"}
    ],"edges":[{"id":"e1","fromNode":"p1","toNode":"p2"},{"id":"e2","fromNode":"t1","toNode":"p2"}],
    "appState":{"zoom":0.5}}"#;

    fn sample() -> CanvasDocument {
        CanvasDocument::from_json_canvas(SAMPLE).unwrap()
    }

    fn saved(document: &CanvasDocument) -> Value {
        serde_json::from_str(&document.to_json_canvas().unwrap()).unwrap()
    }

    fn page(id: &str) -> PageNode {
        PageNode {
            node_id: id.to_owned(),
            url: "https://c.test/".to_owned(),
            rect: CanvasRect::new(-50.0, 900.0, 393.0, 852.0),
        }
    }

    #[test]
    fn pages_reports_a_node_whose_fields_no_longer_parse() {
        let document = sample();
        {
            let mut txn = document.doc.transact_mut();
            let broken = serde_json::json!({"id": "broken", "type": "link", "x": "left"});
            document.insert_node(&mut txn, "broken", &broken);
        }
        assert!(document.pages().is_err());
    }

    #[test]
    fn pages_lists_only_link_nodes_in_order() {
        let ids: Vec<_> = sample()
            .pages()
            .unwrap()
            .into_iter()
            .map(|p| p.node_id)
            .collect();
        assert_eq!(ids, ["p1", "p2"]);
    }

    #[test]
    fn json_round_trip_preserves_canvas_value() {
        let original: Value = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(saved(&sample()), original);
    }

    #[test]
    fn load_rejects_duplicate_node_ids() {
        let json = r#"{"nodes":[
            {"id":"a","type":"text","x":0,"y":0,"width":1,"height":1},
            {"id":"a","type":"text","x":0,"y":0,"width":1,"height":1}]}"#;
        assert!(matches!(
            CanvasDocument::from_json_canvas(json),
            Err(DocumentError::DuplicateNode(_))
        ));
    }

    #[test]
    fn load_rejects_node_missing_geometry() {
        let json = r#"{"nodes":[{"id":"a","type":"text"}]}"#;
        assert!(matches!(
            CanvasDocument::from_json_canvas(json),
            Err(DocumentError::Parse(_))
        ));
    }

    #[test]
    fn loading_is_not_undoable() {
        assert!(!sample().undo());
    }

    #[test]
    fn set_node_rect_moves_page() {
        let mut document = sample();
        let rect = CanvasRect::new(10.0, 20.0, 1280.0, 800.0);
        document.set_node_rect("p1", rect).unwrap();
        assert_eq!(document.pages().unwrap()[0].rect, rect);
    }

    #[test]
    fn set_node_rect_resizes_page() {
        let mut document = sample();
        let rect = CanvasRect::new(0.0, 0.0, 390.0, 844.0);
        document.set_node_rect("p1", rect).unwrap();
        assert_eq!(document.pages().unwrap()[0].rect, rect);
    }

    #[test]
    fn set_node_rect_keeps_whole_coordinates_integral() {
        let mut document = sample();
        document
            .set_node_rect("p1", CanvasRect::new(10.0, 20.0, 30.0, 40.0))
            .unwrap();
        assert_eq!(saved(&document)["nodes"][0]["x"], serde_json::json!(10));
    }

    #[test]
    fn set_node_rect_rejects_unknown_node() {
        let mut document = CanvasDocument::new();
        assert!(matches!(
            document.set_node_rect("missing", CanvasRect::default()),
            Err(DocumentError::UnknownNode(_))
        ));
    }

    #[test]
    fn undo_restores_rect_before_move() {
        let mut document = sample();
        let before = saved(&document);
        document
            .set_node_rect("p1", CanvasRect::new(10.0, 20.0, 30.0, 40.0))
            .unwrap();
        document.undo();
        assert_eq!(saved(&document), before);
    }

    #[test]
    fn redo_reapplies_move() {
        let mut document = sample();
        let rect = CanvasRect::new(10.0, 20.0, 30.0, 40.0);
        document.set_node_rect("p1", rect).unwrap();
        document.undo();
        document.redo();
        assert_eq!(document.pages().unwrap()[0].rect, rect);
    }

    #[test]
    fn rapid_mutations_undo_one_at_a_time() {
        let mut document = sample();
        let first = CanvasRect::new(1.0, 1.0, 1280.0, 800.0);
        document.set_node_rect("p1", first).unwrap();
        document
            .set_node_rect("p1", CanvasRect::new(2.0, 2.0, 1280.0, 800.0))
            .unwrap();
        document.undo();
        assert_eq!(document.pages().unwrap()[0].rect, first);
    }

    #[test]
    fn add_page_appends_front_most_page() {
        let mut document = sample();
        document.add_page(&page("p3")).unwrap();
        assert_eq!(document.pages().unwrap().last(), Some(&page("p3")));
    }

    #[test]
    fn add_page_rejects_duplicate_id() {
        let mut document = sample();
        assert!(matches!(
            document.add_page(&page("t1")),
            Err(DocumentError::DuplicateNode(_))
        ));
    }

    #[test]
    fn undo_add_page_restores_document() {
        let mut document = sample();
        let before = saved(&document);
        document.add_page(&page("p3")).unwrap();
        document.undo();
        assert_eq!(saved(&document), before);
    }

    #[test]
    fn remove_node_drops_attached_edges() {
        let mut document = sample();
        document.remove_node("p2").unwrap();
        assert_eq!(saved(&document)["edges"], serde_json::json!([]));
    }

    #[test]
    fn remove_node_keeps_unattached_edges() {
        let mut document = sample();
        document.remove_node("p1").unwrap();
        assert_eq!(saved(&document)["edges"][0]["id"], "e2");
    }

    #[test]
    fn remove_node_rejects_unknown_node() {
        let mut document = sample();
        assert!(matches!(
            document.remove_node("missing"),
            Err(DocumentError::UnknownNode(_))
        ));
    }

    #[test]
    fn undo_remove_node_restores_node_order_and_edges() {
        let mut document = sample();
        let before = saved(&document);
        document.remove_node("p1").unwrap();
        document.undo();
        assert_eq!(saved(&document), before);
    }

    #[test]
    fn failed_mutation_adds_no_undo_step() {
        let mut document = sample();
        let _ = document.remove_node("missing");
        assert!(!document.undo());
    }
}
