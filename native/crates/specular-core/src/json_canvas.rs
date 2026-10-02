//! JSON Canvas v1.0 on-disk shape (`.canvas` files), lossless.
//!
//! Only the fields the spike reads are typed; every other field (Specular's
//! `specular.*` extensions, `color`, `label`, ...) rides along in `extra` so a
//! load/save round trip never drops data another tool wrote. This is the only
//! module that says "node" — everywhere else a `link` node is a page.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A whole `.canvas` document.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct JsonCanvas {
    /// Canvas items, back-to-front.
    #[serde(default)]
    pub nodes: Vec<Node>,
    /// Connections between nodes.
    #[serde(default)]
    pub edges: Vec<Edge>,
    /// Unknown top-level fields.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// One JSON Canvas node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Stable node id.
    pub id: String,
    /// Node type: `text`, `file`, `link`, `group`, or an extension type.
    #[serde(rename = "type")]
    pub kind: String,
    /// Left edge in canvas units.
    pub x: f64,
    /// Top edge in canvas units.
    pub y: f64,
    /// Width in canvas units.
    pub width: f64,
    /// Height in canvas units.
    pub height: f64,
    /// Every other field, preserved verbatim.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Node {
    /// The URL of a `link` node (a page); `None` for other kinds.
    pub fn link_url(&self) -> Option<&str> {
        if self.kind != "link" {
            return None;
        }
        self.extra.get("url").and_then(Value::as_str)
    }
}

/// One JSON Canvas edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Stable edge id.
    pub id: String,
    /// Source node id.
    #[serde(rename = "fromNode")]
    pub from_node: String,
    /// Target node id.
    #[serde(rename = "toNode")]
    pub to_node: String,
    /// Every other field, preserved verbatim.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "nodes": [
            {"id": "p1", "type": "link", "x": 0, "y": 0, "width": 1280, "height": 800,
             "url": "https://example.com/", "presetIndex": 2},
            {"id": "t1", "type": "text", "x": 10, "y": 900, "width": 200, "height": 80, "text": "hi"}
        ],
        "edges": [{"id": "e1", "fromNode": "p1", "toNode": "t1", "label": "see"}]
    }"#;

    #[test]
    fn link_node_exposes_url() {
        let canvas: JsonCanvas = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(canvas.nodes[0].link_url(), Some("https://example.com/"));
    }

    #[test]
    fn text_node_has_no_link_url() {
        let canvas: JsonCanvas = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(canvas.nodes[1].link_url(), None);
    }

    #[test]
    fn round_trip_preserves_extension_fields() {
        let canvas: JsonCanvas = serde_json::from_str(SAMPLE).unwrap();
        let reparsed: JsonCanvas =
            serde_json::from_str(&serde_json::to_string(&canvas).unwrap()).unwrap();
        assert_eq!(reparsed, canvas);
    }
}
