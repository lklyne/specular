//! The canvas document: JSON Canvas content held in a yrs (Yjs) doc.
//!
//! Mirrors the Electron app's two-layer model in miniature: the yrs doc is the
//! persisted, undoable truth; every mutation is one transaction and one undo
//! step; `.canvas` JSON is a projection of it. Nodes live in a root map keyed
//! by id (each node a nested map of its JSON fields) with a root array keeping
//! back-to-front order, so moving a page touches only its `x`/`y` keys.

use serde_json::Value;
use yrs::any::Number;
use yrs::types::ToJson;
use yrs::{Any, Array, ArrayRef, Doc, In, Map, MapPrelim, MapRef, Out, Transact, UndoManager};

use crate::geometry::CanvasRect;
use crate::json_canvas::{Edge, JsonCanvas, Node};

const NODES: &str = "nodes";
const NODE_ORDER: &str = "nodeOrder";
const EDGES: &str = "edges";

/// Errors loading, saving, or mutating a [`CanvasDocument`].
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// The input is not valid JSON Canvas.
    #[error("invalid JSON Canvas: {0}")]
    Parse(#[from] serde_json::Error),
    /// No node with this id exists.
    #[error("unknown node {0:?}")]
    UnknownNode(String),
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
        let mut undo = UndoManager::new();
        undo.expand_scope(&doc, &nodes);
        undo.expand_scope(&doc, &node_order);
        undo.expand_scope(&doc, &edges);
        Self {
            doc,
            nodes,
            node_order,
            edges,
            undo,
        }
    }

    /// Loads a `.canvas` file's contents. Loading is not an undo step.
    pub fn from_json_canvas(json: &str) -> Result<Self, DocumentError> {
        let canvas: JsonCanvas = serde_json::from_str(json)?;
        let mut document = Self::new();
        {
            let mut txn = document.doc.transact_mut();
            for node in canvas.nodes {
                let id = node.id.clone();
                let fields = to_map_prelim(serde_json::to_value(node)?)?;
                document.nodes.insert(&mut txn, id.as_str(), fields);
                document.node_order.push_back(&mut txn, id);
            }
            for edge in canvas.edges {
                let any: Any = serde_json::from_value(serde_json::to_value(edge)?)?;
                document.edges.push_back(&mut txn, any);
            }
        }
        document.undo.clear_all();
        Ok(document)
    }

    /// Serializes the document back to JSON Canvas (pretty-printed, so the
    /// file stays diffable).
    pub fn to_json_canvas(&self) -> Result<String, DocumentError> {
        let txn = self.doc.transact();
        let mut canvas = JsonCanvas::default();
        for id in self.node_order.iter(&txn) {
            let Out::Any(Any::String(id)) = id else {
                continue;
            };
            let Some(Out::YMap(node)) = self.nodes.get(&txn, &id) else {
                continue;
            };
            let value = serde_json::to_value(node.to_json(&txn))?;
            canvas.nodes.push(serde_json::from_value::<Node>(value)?);
        }
        for edge in self.edges.iter(&txn) {
            let Out::Any(any) = edge else { continue };
            let value = serde_json::to_value(any)?;
            canvas.edges.push(serde_json::from_value::<Edge>(value)?);
        }
        drop(txn);
        Ok(serde_json::to_string_pretty(&canvas)?)
    }

    /// Every page (`link` node), back-to-front.
    pub fn pages(&self) -> Vec<PageNode> {
        let txn = self.doc.transact();
        self.node_order
            .iter(&txn)
            .filter_map(|id| {
                let Out::Any(Any::String(id)) = id else {
                    return None;
                };
                let Some(Out::YMap(node)) = self.nodes.get(&txn, &id) else {
                    return None;
                };
                let value = serde_json::to_value(node.to_json(&txn)).ok()?;
                let node: Node = serde_json::from_value(value).ok()?;
                let url = node.link_url()?.to_owned();
                Some(PageNode {
                    node_id: node.id,
                    url,
                    rect: CanvasRect::new(
                        node.x as f32,
                        node.y as f32,
                        node.width as f32,
                        node.height as f32,
                    ),
                })
            })
            .collect()
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
        // Close the capture window so rapid mutations stay separate undo steps.
        self.undo.reset();
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
}

/// JSON Canvas stores integers; keep whole numbers integral so saved files
/// do not churn `100` into `100.0`.
fn canvas_number(value: f32) -> Any {
    if value.fract() == 0.0 && value.abs() < i64::MAX as f32 {
        Any::Number(Number::Int(value as i64))
    } else {
        Any::Number(Number::Float(f64::from(value)))
    }
}

fn to_map_prelim(value: Value) -> Result<MapPrelim, DocumentError> {
    let Value::Object(fields) = value else {
        return Ok(MapPrelim::default());
    };
    fields
        .into_iter()
        .map(|(key, field)| Ok((key, In::Any(serde_json::from_value::<Any>(field)?))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{"nodes":[
        {"id":"p1","type":"link","x":0,"y":0,"width":1280,"height":800,"url":"https://a.test/","presetIndex":1},
        {"id":"t1","type":"text","x":5,"y":5,"width":10,"height":10,"text":"note"},
        {"id":"p2","type":"link","x":1400,"y":0,"width":390,"height":844,"url":"https://b.test/"}
    ],"edges":[{"id":"e1","fromNode":"p1","toNode":"p2"}]}"#;

    #[test]
    fn pages_lists_only_link_nodes_in_order() {
        let document = CanvasDocument::from_json_canvas(SAMPLE).unwrap();
        let ids: Vec<_> = document.pages().into_iter().map(|p| p.node_id).collect();
        assert_eq!(ids, ["p1", "p2"]);
    }

    #[test]
    fn json_round_trip_preserves_canvas() {
        let document = CanvasDocument::from_json_canvas(SAMPLE).unwrap();
        let original: JsonCanvas = serde_json::from_str(SAMPLE).unwrap();
        let saved: JsonCanvas = serde_json::from_str(&document.to_json_canvas().unwrap()).unwrap();
        assert_eq!(saved, original);
    }

    #[test]
    fn set_node_rect_moves_page() {
        let mut document = CanvasDocument::from_json_canvas(SAMPLE).unwrap();
        let rect = CanvasRect::new(10.0, 20.0, 1280.0, 800.0);
        document.set_node_rect("p1", rect).unwrap();
        assert_eq!(document.pages()[0].rect, rect);
    }

    #[test]
    fn undo_restores_rect_before_move() {
        let mut document = CanvasDocument::from_json_canvas(SAMPLE).unwrap();
        let before = document.pages()[0].rect;
        document
            .set_node_rect("p1", CanvasRect::new(10.0, 20.0, 30.0, 40.0))
            .unwrap();
        document.undo();
        assert_eq!(document.pages()[0].rect, before);
    }

    #[test]
    fn loading_is_not_undoable() {
        let mut document = CanvasDocument::from_json_canvas(SAMPLE).unwrap();
        assert!(!document.undo());
    }

    #[test]
    fn set_node_rect_rejects_unknown_node() {
        let mut document = CanvasDocument::new();
        assert!(matches!(
            document.set_node_rect("missing", CanvasRect::default()),
            Err(DocumentError::UnknownNode(_))
        ));
    }
}
