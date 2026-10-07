//! [`Annotation`]: a comment thread pinned to the canvas, a page, an element
//! or a region. Its fields match the objects in the `.canvas` file's
//! `annotations` array, so it derives serde directly.

use serde::{Deserialize, Serialize};

use crate::{AnnotationId, EntityId, JsonMap, PageAnchor, Rect};

/// A comment thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    /// Stable id.
    pub id: AnnotationId,
    /// Where the annotation is pinned.
    pub anchor: AnnotationAnchor,
    /// Who wrote it.
    pub author: Author,
    /// The comment.
    pub text: String,
    /// Where the thread stands.
    pub status: AnnotationStatus,
    /// Replies, oldest first.
    pub replies: Vec<Reply>,
    /// Creation time, ISO 8601.
    pub created_at: String,
    /// A user-curated name for the anchored element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_name: Option<String>,
    /// Present when the annotation is bound to a page's document. Written
    /// once at creation and never re-resolved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_anchor: Option<PageAnchor>,
    /// Inspect context, screenshots, agent session ids and the like.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonMap>,
    /// Unmodeled fields.
    #[serde(flatten)]
    pub extra: JsonMap,
}

/// Where an annotation is pinned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "lowercase",
    rename_all_fields = "camelCase"
)]
pub enum AnnotationAnchor {
    /// A point on the canvas.
    Canvas {
        /// Canvas x.
        canvas_x: f64,
        /// Canvas y.
        canvas_y: f64,
    },
    /// A point on a page, relative to the page's rect.
    Page {
        /// The page.
        page_id: EntityId,
        /// Offset from the page's left edge.
        offset_x: f64,
        /// Offset from the page's top edge.
        offset_y: f64,
    },
    /// A DOM element in a page.
    Element {
        /// The page.
        page_id: EntityId,
        /// DOM selector for the element.
        selector: String,
        /// A readable path to the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        element_path: Option<String>,
        /// The element's box when the annotation was made.
        #[serde(skip_serializing_if = "Option::is_none")]
        bounding_box: Option<Rect>,
    },
    /// A marquee region.
    Region(RegionAnchor),
}

/// The two forms of a region annotation, split by whether the marquee
/// grabbed page content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RegionAnchor {
    /// A region of canvas space.
    Canvas {
        /// The region in canvas units.
        #[serde(rename = "canvasRect")]
        canvas_rect: Rect,
    },
    /// A region of a page's document, in that page's CSS pixels. The page is
    /// the one named by [`Annotation::page_anchor`].
    Document {
        /// The region in document CSS pixels.
        #[serde(rename = "docRect")]
        doc_rect: Rect,
    },
}

/// Who wrote an annotation or reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Author {
    /// The user.
    User,
    /// An agent.
    Agent,
}

/// Where an annotation thread stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnnotationStatus {
    /// Not yet seen by an agent.
    Pending,
    /// Seen, not yet handled.
    Acknowledged,
    /// Handled.
    Resolved,
    /// Closed without action.
    Dismissed,
}

/// One reply in an annotation thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    /// Who wrote it.
    pub author: Author,
    /// The reply.
    pub text: String,
    /// When it was written, ISO 8601.
    pub timestamp: String,
    /// Unmodeled fields.
    #[serde(flatten)]
    pub extra: JsonMap,
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    fn annotation_with(anchor: &Value) -> Value {
        json!({
            "id": "a1", "anchor": anchor, "author": "user", "text": "fix this",
            "status": "pending", "createdAt": "2026-01-01T00:00:00.000Z",
            "replies": [{"author": "agent", "text": "done", "timestamp": "2026-01-01T00:01:00.000Z"}],
            "pageAnchor": {"pageId": "p1", "pageUrl": "https://example.com/", "scrollY": 120.5,
                "element": {"selector": "#cta", "docX": 10.0, "docY": 20.0}},
            "metadata": {"pageName": "iPad Mini"},
            "otherTool": true,
        })
    }

    #[test]
    fn every_anchor_form_round_trips() {
        let anchors = [
            json!({"type": "canvas", "canvasX": 1.5, "canvasY": -2.0}),
            json!({"type": "page", "pageId": "p1", "offsetX": 3.0, "offsetY": 4.0}),
            json!({"type": "element", "pageId": "p1", "selector": "#cta", "elementPath": "main > a",
                "boundingBox": {"x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}}),
            json!({"type": "element", "pageId": "p1", "selector": "#cta"}),
            json!({"type": "region", "canvasRect": {"x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}}),
            json!({"type": "region", "docRect": {"x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0}}),
        ];
        for anchor in anchors {
            let json = annotation_with(&anchor);
            let annotation: Annotation = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(serde_json::to_value(&annotation).unwrap(), json);
        }
    }

    #[test]
    fn region_anchor_forms_are_told_apart() {
        let json = annotation_with(&json!({
            "type": "region", "docRect": {"x": 0.0, "y": 0.0, "width": 1.0, "height": 1.0},
        }));
        let annotation: Annotation = serde_json::from_value(json).unwrap();
        assert!(matches!(
            annotation.anchor,
            AnnotationAnchor::Region(RegionAnchor::Document { .. })
        ));
    }
}
