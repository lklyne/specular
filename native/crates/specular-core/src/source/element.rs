//! What a page says about one of its elements.

use glam::Vec2;

use crate::geometry::PixelRect;

/// A DOM element a page found under a point: what a comment on it records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageElement {
    /// A CSS selector that finds the element in its document.
    pub selector: String,
    /// A readable path to the element, when the backend builds one.
    pub element_path: Option<String>,
    /// The element's box in the page's viewport, in CSS pixels.
    pub bounding_box: PixelRect,
}

/// A DOM node as the inspect tool reads it: what its outline, its popover
/// and the chat's pill show (`inspectionPayload` in the Electron app).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectedNode {
    /// What names the node within its page: its `id` attribute, else its
    /// `data-testid`, else `tag@left:top`.
    pub node_id: String,
    /// The tag name, in lower case.
    pub tag_name: String,
    /// The tag and the node's text in quotes (`button "Save"`), or the tag
    /// alone when it has no text, label, title, value, placeholder or alt.
    pub name: String,
    /// A CSS selector that finds the node in its document.
    pub selector: String,
    /// The `id` attribute, when it has one.
    pub id_attribute: Option<String>,
    /// The node's classes, in document order.
    pub classes: Vec<String>,
    /// Computed styles as `(property, value)`, in this order: `display`,
    /// `position`, `font-family`, `font-size`, `font-weight`, `color`,
    /// `background`, `padding`, `margin`. `background` is the computed
    /// background colour.
    pub styles: Vec<(String, String)>,
    /// The node's box in the page's viewport, in CSS pixels.
    pub bounding_box: PixelRect,
}

impl InspectedNode {
    /// The computed value of `property`, when the page reported it.
    pub fn style(&self, property: &str) -> Option<&str> {
        self.styles
            .iter()
            .find(|(name, _)| name == property)
            .map(|(_, value)| value.as_str())
    }
}

/// Where an element sits in its page's document.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ElementPlace {
    /// The top-left corner of the element's box, in document CSS pixels.
    pub doc: Vec2,
    /// Whether the element sits in a fixed or sticky container, so that its
    /// place in the document moves as the page scrolls.
    pub viewport_positioned: bool,
}

/// The element a page offers for an item placed over it to follow
/// (ADR 0032): what a [`PageSource::capture_element`] finds.
#[derive(Debug, Clone, PartialEq)]
pub struct CapturedElement {
    /// A CSS selector that finds the element in its document.
    pub selector: String,
    /// Where the element is now.
    pub place: ElementPlace,
}
