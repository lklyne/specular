//! [`Entity`]: one canvas item, and [`Kind`]: what sort of item it is.

use crate::{Drawing, EntityId, FileRef, Group, JsonMap, Page, PageAnchor, Rect, Shape, Text};

/// One canvas item. The fields here are common to every kind; the rest live
/// in [`Kind`].
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Stable id.
    pub id: EntityId,
    /// The entity rect in canvas space.
    pub rect: Rect,
    /// The sidebar name (a group's title, a page's name), distinct from the
    /// item's rendered content.
    pub label: Option<String>,
    /// The page this item is hooked to, if any.
    pub anchor: Option<PageAnchor>,
    /// The group this item belongs to, if any.
    pub parent: Option<EntityId>,
    /// The kind and its fields.
    pub kind: Kind,
    /// Unmodeled node fields.
    pub extra: JsonMap,
}

impl Entity {
    /// A free-form, ungrouped, unnamed entity.
    pub fn new(id: impl Into<EntityId>, rect: Rect, kind: Kind) -> Self {
        Self {
            id: id.into(),
            rect,
            label: None,
            anchor: None,
            parent: None,
            kind,
            extra: JsonMap::new(),
        }
    }
}

/// The six kinds of canvas item. Match on this without a wildcard arm, so a
/// new kind makes the compiler list every place that must handle it.
#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// A live web page.
    Page(Page),
    /// Plain text or a sticky note.
    Text(Text),
    /// A file from the space folder.
    File(FileRef),
    /// A group of other entities.
    Group(Group),
    /// A freehand drawing.
    Drawing(Drawing),
    /// A geometric shape.
    Shape(Shape),
}

impl Kind {
    /// The kind's name as the CLI and error messages spell it.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Page(_) => "page",
            Self::Text(_) => "text",
            Self::File(_) => "file",
            Self::Group(_) => "group",
            Self::Drawing(_) => "drawing",
            Self::Shape(_) => "shape",
        }
    }
}
