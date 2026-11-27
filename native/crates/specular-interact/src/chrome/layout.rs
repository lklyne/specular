//! [`Layout`]: one layout file, `layouts/<name>.json` (ADR 0059): the
//! window as a tree of nodes, its look, and what fills what lies over it.
//!
//! ```json
//! { "type": "column", "id": "root", "theme": "dark", "font": "Inter",
//!   "toast": { "mod": "toast" },
//!   "children": [
//!     { "id": "tabs", "type": "block", "size": { "px": 38 }, "content": { "mod": "tabs" } },
//!     { "id": "body", "type": "row", "children": [
//!       { "id": "sidebar", "type": "block", "size": { "px": 256 }, "content": { "panel": "sidebar" } },
//!       { "id": "canvas", "type": "block", "content": "canvas" } ] } ] }
//! ```
//!
//! The root is a node like any other, with the look (`theme`, `colors`,
//! `font`, `font_size`, `background`) and the overlays (`dock`, `menu`,
//! `share`, `toast`, `palette`, `welcome`) beside its own fields. An overlay
//! left out is the app's own. A file in the five-region shape ADR 0055
//! wrote is read as the tree it means. A file that does not read as a
//! layout is an error with the reason, and [`LayoutFile`] keeps the last
//! good layout in force past it.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::look::Look;
use super::node::{Content, Node, NodeKind, lists};
use super::{Overlay, Region, SIDEBAR_WIDTH, TAB_ROW, TOOLS_DOCK_WIDTH};

/// What fills an overlay, and whether it is drawn.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionNode {
    /// The mod that fills it, or `None` for the app's own.
    #[serde(rename = "mod", default)]
    pub fill: Option<String>,
    /// Whether it is left out.
    #[serde(default)]
    pub hidden: bool,
    /// What the mod that fills it is handed, untouched.
    #[serde(default)]
    pub props: Map<String, Value>,
}

/// One whole window.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// What the layout is called where a person picks it.
    pub title: Option<String>,
    /// The tree.
    pub root: Node,
    /// The look its root sets.
    pub look: Look,
    /// What fills each overlay, in [`Overlay::ALL`]'s order.
    overlays: [RegionNode; 6],
}

impl Default for Layout {
    fn default() -> Self {
        Self::shipped()
    }
}

/// Why a file is not a layout, or not a set of toggles.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct LayoutError(pub String);

impl From<serde_json::Error> for LayoutError {
    fn from(error: serde_json::Error) -> Self {
        Self(error.to_string())
    }
}

/// The keys only a layout's root has.
const LOOK_KEYS: [&str; 6] = [
    "title",
    "theme",
    "colors",
    "font",
    "font_size",
    "background",
];

/// The layouts the app ships, by name, as their files hold them.
pub const SHIPPED: [(&str, &str); 4] = [
    ("default", include_str!("../../layouts/default.json")),
    ("classic", include_str!("../../layouts/classic.json")),
    ("studio", include_str!("../../layouts/studio.json")),
    ("mods", include_str!("../../layouts/mods.json")),
];

/// The themes the app ships, by name, as their files hold them.
pub const SHIPPED_THEMES: [(&str, &str); 1] =
    [("studio", include_str!("../../themes/studio.json"))];

impl Layout {
    /// The layout the app shows when the person picked none, `default`.
    pub fn shipped() -> Self {
        Self::parse(SHIPPED[0].1).unwrap_or_else(|_| Self::classic())
    }

    /// The app's own panels in today's places: the tab row across the top;
    /// the sidebar, the docked tools, the canvas and the right panel under
    /// it; every overlay the app's own.
    pub fn classic() -> Self {
        let panel = |region: Region| Node::block(Content::Panel(region)).with_id(region.name());
        let body = Node::line(
            NodeKind::Row,
            vec![
                panel(Region::Sidebar).px(SIDEBAR_WIDTH),
                panel(Region::Toolbar).px(TOOLS_DOCK_WIDTH),
                Node::block(Content::Canvas).with_id("canvas"),
                panel(Region::Chat).px(crate::chat::CHAT_WIDTH),
            ],
        )
        .with_id("body");
        let root = Node::line(
            NodeKind::Column,
            vec![panel(Region::Tabs).px(TAB_ROW), body],
        )
        .with_id("root");
        Self {
            title: Some("Classic".to_owned()),
            root,
            look: Look::default(),
            overlays: Default::default(),
        }
    }

    /// The layout `text` says, or why it says none.
    pub fn parse(text: &str) -> Result<Self, LayoutError> {
        let value: Value = serde_json::from_str(text)?;
        let Value::Object(mut fields) = value else {
            return Err(LayoutError(
                "invalid type: a layout is a JSON object".to_owned(),
            ));
        };
        if !fields.contains_key("type") {
            return super::legacy::read(fields);
        }
        let mut overlays: [RegionNode; 6] = Default::default();
        for (overlay, slot) in Overlay::ALL.iter().zip(overlays.iter_mut()) {
            if let Some(node) = fields.remove(overlay.name()) {
                *slot = serde_json::from_value(node)
                    .map_err(|error| LayoutError(format!("`{}`: {error}", overlay.name())))?;
            }
        }
        let mut look = Map::new();
        for key in LOOK_KEYS {
            if let Some(value) = fields.remove(key) {
                look.insert(key.to_owned(), value);
            }
        }
        let title = match look.remove("title") {
            Some(Value::String(title)) => Some(title),
            Some(_) => return Err(LayoutError("`title` is text".to_owned())),
            None => None,
        };
        let look: Look = serde_json::from_value(Value::Object(look))?;
        let root: Node = serde_json::from_value(Value::Object(fields))?;
        let layout = Self {
            title,
            root,
            look,
            overlays,
        };
        layout.check()?;
        Ok(layout)
    }

    /// Ids are unique and there is one canvas.
    fn check(&self) -> Result<(), LayoutError> {
        let nodes = self.root.walk();
        let mut seen = std::collections::BTreeSet::new();
        for id in nodes.iter().filter_map(|node| node.id.as_deref()) {
            if !seen.insert(id) {
                return Err(LayoutError(format!("two nodes have the id `{id}`")));
            }
        }
        let canvases = (nodes.iter())
            .filter(|node| node.content == Some(Content::Canvas))
            .count();
        if canvases != 1 {
            return Err(LayoutError(format!(
                "a layout has one block with \"content\": \"canvas\", and this has {canvases}"
            )));
        }
        Ok(())
    }

    /// The layout as a file holds it.
    pub fn to_text(&self) -> String {
        let mut fields = match serde_json::to_value(&self.root) {
            Ok(Value::Object(fields)) => fields,
            _ => Map::new(),
        };
        if let Some(title) = &self.title {
            fields.insert("title".to_owned(), Value::String(title.clone()));
        }
        if let Ok(Value::Object(look)) = serde_json::to_value(&self.look) {
            fields.extend(look);
        }
        for (overlay, node) in Overlay::ALL.iter().zip(&self.overlays) {
            if *node != RegionNode::default() {
                let value = serde_json::to_value(node).unwrap_or(Value::Null);
                fields.insert(overlay.name().to_owned(), value);
            }
        }
        let mut text = serde_json::to_string_pretty(&Value::Object(fields)).unwrap_or_default();
        text.push('\n');
        text
    }

    /// What fills `overlay`.
    pub fn overlay(&self, overlay: Overlay) -> &RegionNode {
        &self.overlays[overlay.index()]
    }

    /// What fills `overlay`, to change.
    pub fn overlay_mut(&mut self, overlay: Overlay) -> &mut RegionNode {
        &mut self.overlays[overlay.index()]
    }

    /// The node with the id `id`.
    pub fn node(&self, id: &str) -> Option<&Node> {
        self.root.find(id)
    }

    /// The mod that fills `region`: the mod of the block with its name, or
    /// the dock overlay's.
    pub fn fill(&self, region: Region) -> Option<&str> {
        match region {
            Region::Dock => self.overlay(Overlay::Dock).fill.as_deref(),
            _ => self
                .node(region.name())
                .and_then(Node::mod_content)
                .map(|(name, _)| name),
        }
    }

    /// Every mod the layout places: the name its copy goes by (the block's
    /// id, else the mod's own; the overlay's name), the mod and its props.
    pub fn mods(&self) -> Vec<(String, &str, &Map<String, Value>)> {
        let blocks = (self.root.walk().into_iter()).filter_map(|node| {
            let (name, props) = node.mod_content()?;
            Some((
                node.id.clone().unwrap_or_else(|| name.to_owned()),
                name,
                props,
            ))
        });
        let overlays = (Overlay::ALL.iter().zip(&self.overlays)).filter_map(|(overlay, node)| {
            let name = node.fill.as_deref()?;
            Some((overlay.name().to_owned(), name, &node.props))
        });
        blocks.chain(overlays).collect()
    }

    /// Whether the layout hides `id`: a node or an overlay by its
    /// `hidden`, a control by a mod's `props.hidden` listing it.
    pub fn hides(&self, id: &str) -> bool {
        if let Some(node) = self.node(id) {
            return node.hidden;
        }
        if let Some(overlay) = Overlay::named(id) {
            return self.overlay(overlay).hidden;
        }
        self.root.walk().iter().any(|node| node.lists_hidden(id))
            || self.overlays.iter().any(|node| lists(&node.props, id))
    }

    /// Whether `id` names a node or an overlay of the layout.
    pub fn names(&self, id: &str) -> bool {
        self.node(id).is_some() || Overlay::named(id).is_some()
    }
}

/// A layout file read again and again: the last layout that read well, and
/// why the latest read did not.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LayoutFile {
    good: Layout,
    error: Option<LayoutError>,
}

impl LayoutFile {
    /// A file whose last good layout is `layout`.
    pub fn new(layout: Layout) -> Self {
        Self {
            good: layout,
            error: None,
        }
    }

    /// Reads `text`. A layout replaces the one in force; anything else
    /// leaves it in force and is kept as the error. Returns the error.
    pub fn read(&mut self, text: &str) -> Option<&LayoutError> {
        match Layout::parse(text) {
            Ok(layout) => {
                self.good = layout;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
        self.error.as_ref()
    }

    /// The layout in force.
    pub const fn layout(&self) -> &Layout {
        &self.good
    }

    /// Why the latest read was not a layout, if it was not.
    pub const fn error(&self) -> Option<&LayoutError> {
        self.error.as_ref()
    }
}
