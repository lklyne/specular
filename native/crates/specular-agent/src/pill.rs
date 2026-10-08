use crate::text::truncate;

/// What the composer's pill points at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pill {
    /// A DOM node of a page.
    Dom {
        /// The element's name.
        label: String,
        /// The page's origin.
        origin: Option<String>,
        /// The page entity.
        page_id: Option<String>,
    },
    /// A focused comment.
    Annotation {
        /// What to show.
        label: String,
        /// The comment.
        annotation_id: String,
    },
    /// Selected canvas items.
    Selection {
        /// What to show.
        label: String,
        /// The selected entities.
        entity_ids: Vec<String>,
    },
    /// Nothing is picked.
    Empty,
}

/// An inspected DOM node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectNode {
    /// Accessible name.
    pub name: String,
    /// Tag name.
    pub tag_name: String,
    /// The page's origin.
    pub origin: Option<String>,
    /// The page entity.
    pub page_id: Option<String>,
}

/// A focused comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusedAnnotation {
    /// The comment id.
    pub id: String,
    /// Its text.
    pub text: String,
    /// The element it is pinned to.
    pub element_name: Option<String>,
    /// `element`, `region`, `page`, or other.
    pub anchor_type: String,
}

/// The canvas selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasSelection {
    /// How many items.
    pub count: usize,
    /// What to show.
    pub label: String,
    /// The entities.
    pub entity_ids: Vec<String>,
}

/// Everything the pill is resolved from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PillInput {
    /// The inspected DOM node.
    pub inspect_node: Option<InspectNode>,
    /// The focused comment.
    pub focused_annotation: Option<FocusedAnnotation>,
    /// The selected items.
    pub canvas_selection: Option<CanvasSelection>,
}

/// The pill for an input: DOM node, then focused comment, then selection.
pub fn resolve(input: &PillInput) -> Pill {
    if let Some(node) = &input.inspect_node {
        let label = [node.name.trim(), node.tag_name.trim(), "element"]
            .into_iter()
            .find(|s| !s.is_empty())
            .unwrap_or("element");
        return Pill::Dom {
            label: label.to_owned(),
            origin: node.origin.clone(),
            page_id: node.page_id.clone(),
        };
    }
    if let Some(a) = &input.focused_annotation {
        let snippet = a.text.split_whitespace().collect::<Vec<_>>().join(" ");
        let named = a
            .element_name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let label = match named {
            Some(name) => name.to_owned(),
            None if !snippet.is_empty() => truncate(&snippet, 48),
            None => match a.anchor_type.as_str() {
                "element" => "Element",
                "region" => "Region",
                "page" => "Page",
                _ => "Comment",
            }
            .to_owned(),
        };
        return Pill::Annotation {
            label,
            annotation_id: a.id.clone(),
        };
    }
    match &input.canvas_selection {
        Some(s) if s.count > 0 => Pill::Selection {
            label: s.label.clone(),
            entity_ids: s.entity_ids.clone(),
        },
        _ => Pill::Empty,
    }
}

impl Pill {
    /// The pill's text; the empty pill reads the canvas name.
    pub fn label(&self, canvas_name: &str) -> String {
        match self {
            Self::Dom { label, .. }
            | Self::Annotation { label, .. }
            | Self::Selection { label, .. } => label.clone(),
            Self::Empty if canvas_name.trim().is_empty() => "specular".to_owned(),
            Self::Empty => canvas_name.to_owned(),
        }
    }
}

/// A soft focus for the prompt. Not a write fence.
pub fn focus_prompt(pill: &Pill) -> Option<String> {
    match pill {
        Pill::Selection { entity_ids, .. } if !entity_ids.is_empty() => Some(format!(
            "The user has selected {} and likely wants to focus on those.",
            entity_ids.join(", ")
        )),
        Pill::Annotation { annotation_id, .. } => Some(format!(
            "The user has selected comment {annotation_id} and likely wants to focus on that."
        )),
        Pill::Dom { label, page_id, .. } => {
            let place = page_id
                .as_ref()
                .map(|id| format!(" on page {id}"))
                .unwrap_or_default();
            Some(format!(
                "The user has selected DOM node \"{label}\"{place} and likely wants to focus on that."
            ))
        }
        Pill::Selection { .. } | Pill::Empty => None,
    }
}
