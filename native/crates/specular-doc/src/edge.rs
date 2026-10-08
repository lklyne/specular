//! [`Edge`]: a connection between two entities. Its fields match the
//! `.canvas` edge object, so it derives serde directly.

use serde::{Deserialize, Serialize};

use crate::{Color, EdgeId, EntityId, JsonMap};

/// A connection between two entities.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Edge {
    /// Stable id, in the same namespace as entity ids.
    pub id: EdgeId,
    /// The entity the edge starts at.
    #[serde(rename = "fromNode")]
    pub from: EntityId,
    /// The entity the edge ends at.
    #[serde(rename = "toNode")]
    pub to: EntityId,
    /// Which side of `from` the edge leaves. Absent picks the nearest side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_side: Option<EdgeSide>,
    /// Which side of `to` the edge enters. Absent picks the nearest side.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_side: Option<EdgeSide>,
    /// Endpoint shape at `from`. Absent means none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_end: Option<EdgeEnd>,
    /// Endpoint shape at `to`. Absent means arrow.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_end: Option<EdgeEnd>,
    /// Line color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Text drawn along the edge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Line width in pixels.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
    /// Solid or dashed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_style: Option<LineStyle>,
    /// What the edge means.
    #[serde(rename = "edgeKind", skip_serializing_if = "Option::is_none")]
    pub kind: Option<EdgeKind>,
    /// Edge metadata.
    #[serde(rename = "edgeMetadata", skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonMap>,
    /// Unmodeled fields.
    #[serde(flatten)]
    pub extra: JsonMap,
}

impl Edge {
    /// An unstyled edge between two entities.
    pub fn new(id: impl Into<EdgeId>, from: impl Into<EntityId>, to: impl Into<EntityId>) -> Self {
        Self {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            from_side: None,
            to_side: None,
            from_end: None,
            to_end: None,
            color: None,
            label: None,
            stroke_width: None,
            line_style: None,
            kind: None,
            metadata: None,
            extra: JsonMap::new(),
        }
    }

    /// Whether the edge starts or ends at `id`.
    pub fn touches(&self, id: &EntityId) -> bool {
        self.from == *id || self.to == *id
    }
}

/// A side of an entity rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeSide {
    /// Top side.
    Top,
    /// Right side.
    Right,
    /// Bottom side.
    Bottom,
    /// Left side.
    Left,
}

/// The shape drawn at an edge endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeEnd {
    /// Nothing.
    None,
    /// An arrowhead.
    Arrow,
}

/// An edge's line style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineStyle {
    /// Continuous line.
    Solid,
    /// Dashed line.
    Dashed,
}

/// What an edge means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// Links a page to the same page at another breakpoint.
    BreakpointVariant,
    /// A connection the user drew.
    Connection,
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    #[test]
    fn edge_json_round_trips_with_unmodeled_fields() {
        let json = json!({
            "id": "e1", "fromNode": "a", "toNode": "b",
            "fromSide": "right", "toEnd": "arrow", "color": "4",
            "strokeWidth": 2.5, "lineStyle": "dashed",
            "edgeKind": "breakpoint_variant", "edgeMetadata": {"k": 1},
            "otherTool": [1, 2],
        });
        let edge: Edge = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(edge.kind, Some(EdgeKind::BreakpointVariant));
        assert_eq!(edge.extra.get("otherTool"), Some(&json!([1, 2])));
        assert_eq!(serde_json::to_value(&edge).unwrap(), json);

        {
            let value: Value = serde_json::to_value(Edge::new("e", "a", "b")).unwrap();
            assert_eq!(value, json!({"id": "e", "fromNode": "a", "toNode": "b"}));
        }
    }
}
