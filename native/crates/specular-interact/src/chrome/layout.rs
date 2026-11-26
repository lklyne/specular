//! [`Layout`]: the layout file, which names the window's regions, what fills
//! each and whether it is drawn.
//!
//! ```json
//! {
//!   "tabs":    { "mod": null, "hidden": false, "props": {} },
//!   "toolbar": { "mod": null, "hidden": false, "props": { "hidden": ["tool.inspect"] } },
//!   "dock":    { "mod": null, "hidden": false, "props": {} },
//!   "sidebar": { "mod": null, "hidden": false, "props": {} },
//!   "chat":    { "mod": null, "hidden": false, "props": {} }
//! }
//! ```
//!
//! A region left out is drawn with the app's own panel. `mod` names the mod
//! that fills a region, `null` for the app's own. `props` is handed to that
//! mod untouched; its `hidden` lists the ids of controls the mod leaves out.
//! A file that does not read as a layout is an error with the reason, and
//! [`LayoutFile`] keeps the last good layout in force past it.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::Region;

/// What fills one region, and whether it is drawn.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionNode {
    /// The mod that fills it, or `None` for the app's own panel.
    #[serde(rename = "mod", default)]
    pub fill: Option<String>,
    /// Whether the region is left out, its room given to the canvas.
    #[serde(default)]
    pub hidden: bool,
    /// What the mod that fills it is handed, untouched.
    #[serde(default)]
    pub props: Map<String, Value>,
}

impl RegionNode {
    /// Whether its `props.hidden` lists `id`.
    fn lists_hidden(&self, id: &str) -> bool {
        (self.props.get("hidden").and_then(Value::as_array))
            .is_some_and(|ids| ids.iter().any(|listed| listed.as_str() == Some(id)))
    }
}

/// The window's regions as a layout file names them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    /// The tab row.
    #[serde(default)]
    pub tabs: RegionNode,
    /// The tools.
    #[serde(default)]
    pub toolbar: RegionNode,
    /// The dock.
    #[serde(default)]
    pub dock: RegionNode,
    /// The sidebar.
    #[serde(default)]
    pub sidebar: RegionNode,
    /// The right panel.
    #[serde(default)]
    pub chat: RegionNode,
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

impl Layout {
    /// The layout the app ships: every region drawn, each with the app's
    /// own panel, and the toolbar's `props.hidden` there to be filled in.
    pub fn shipped() -> Self {
        let mut toolbar = RegionNode::default();
        toolbar
            .props
            .insert("hidden".to_owned(), Value::Array(Vec::new()));
        Self {
            tabs: RegionNode::default(),
            toolbar,
            dock: RegionNode::default(),
            sidebar: RegionNode::default(),
            chat: RegionNode::default(),
        }
    }

    /// The layout `text` says, or why it says none.
    pub fn parse(text: &str) -> Result<Self, LayoutError> {
        Ok(serde_json::from_str(text)?)
    }

    /// The layout as a file holds it, one region a block.
    pub fn to_text(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_default();
        text.push('\n');
        text
    }

    /// What fills `region`.
    pub const fn node(&self, region: Region) -> &RegionNode {
        match region {
            Region::Tabs => &self.tabs,
            Region::Toolbar => &self.toolbar,
            Region::Dock => &self.dock,
            Region::Sidebar => &self.sidebar,
            Region::Chat => &self.chat,
        }
    }

    /// Whether the layout hides `id`: a region by its node's `hidden`, a
    /// control by a region's `props.hidden` listing it.
    pub fn hides(&self, id: &str) -> bool {
        match Region::named(id) {
            Some(region) => self.node(region).hidden,
            None => (Region::ALL.iter()).any(|region| self.node(*region).lists_hidden(id)),
        }
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

    /// Goes back to the shipped layout, with no error.
    pub fn reset(&mut self) {
        *self = Self::default();
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
