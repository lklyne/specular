//! Page anchoring: the hook that ties an entity or annotation to a page and
//! the document that page showed when the item was placed.

use serde::{Deserialize, Serialize};

use crate::{EntityId, JsonMap};

/// Ties an item to a page. Anchored entities keep canvas coordinates; the
/// anchor says which page owns the item and which document it belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageAnchor {
    /// The page entity the item is anchored to.
    pub page_id: EntityId,
    /// The page's URL at placement, hash stripped. Absent when the page had
    /// no URL yet; such an anchor matches any document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_url: Option<String>,
    /// Page scroll offset at placement, in page CSS pixels. Present only on
    /// kinds that follow the page's scroll.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_x: Option<f64>,
    /// See [`scroll_x`](Self::scroll_x).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scroll_y: Option<f64>,
    /// The DOM element the item is attached to, when one was captured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element: Option<AnchorElement>,
    /// Unmodeled fields.
    #[serde(flatten)]
    pub extra: JsonMap,
}

impl PageAnchor {
    /// An anchor to `page_id` with no URL, scroll or element recorded.
    pub fn new(page_id: EntityId) -> Self {
        Self {
            page_id,
            page_url: None,
            scroll_x: None,
            scroll_y: None,
            element: None,
            extra: JsonMap::new(),
        }
    }
}

/// An anchored item's reference element and where it sat in the document
/// when captured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorElement {
    /// DOM selector for the element.
    pub selector: String,
    /// The element's document x at capture.
    pub doc_x: f64,
    /// The element's document y at capture.
    pub doc_y: f64,
    /// Whether the element sits in a fixed or sticky container, which
    /// already accounts for scrolling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport_positioned: Option<bool>,
    /// Unmodeled fields.
    #[serde(flatten)]
    pub extra: JsonMap,
}
