//! One node of a layout's tree (ADR 0059): a row, a column, a stack that
//! shows one child, or a block that shows content.
//!
//! ```json
//! { "id": "sidebar", "type": "block", "size": { "px": 256 },
//!   "content": { "mod": "sidebar", "props": {} }, "background": "sidebar" }
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::Region;

/// What kind of node it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    /// Its children side by side.
    Row,
    /// Its children one under another.
    Column,
    /// One child at a time: the one `active` names, else the first.
    Stack,
    /// Shows its `content`.
    Block,
}

/// What a block shows.
#[derive(Debug, Clone, PartialEq)]
pub enum Content {
    /// Where the canvas is seen.
    Canvas,
    /// A mod, and what it is handed.
    Mod {
        /// The mod's folder name.
        name: String,
        /// Handed to the mod untouched, as `props()`.
        props: Map<String, Value>,
    },
    /// One of the app's own panels.
    Panel(Region),
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum ContentText {
    Word(String),
    Mod {
        #[serde(rename = "mod")]
        name: String,
        #[serde(default, skip_serializing_if = "Map::is_empty")]
        props: Map<String, Value>,
    },
    Panel {
        panel: String,
    },
}

impl Serialize for Content {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let text = match self {
            Self::Canvas => ContentText::Word("canvas".to_owned()),
            Self::Mod { name, props } => ContentText::Mod {
                name: name.clone(),
                props: props.clone(),
            },
            Self::Panel(region) => ContentText::Panel {
                panel: region.name().to_owned(),
            },
        };
        text.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;
        match ContentText::deserialize(deserializer)? {
            ContentText::Word(word) if word == "canvas" => Ok(Self::Canvas),
            ContentText::Word(word) => Err(D::Error::custom(format!(
                "content `{word}` is not \"canvas\", a {{\"mod\": ..}} or a {{\"panel\": ..}}"
            ))),
            ContentText::Mod { name, props } => Ok(Self::Mod { name, props }),
            ContentText::Panel { panel } => Region::named(&panel)
                .filter(|region| *region != Region::Dock)
                .map(Self::Panel)
                .ok_or_else(|| {
                    D::Error::custom(format!(
                        "no panel `{panel}`: the app's own are tabs, toolbar, sidebar and chat"
                    ))
                }),
        }
    }
}

/// How much room a node takes along its parent's axis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum Size {
    /// Exactly this many points.
    Px(f32),
    /// A share of what the fixed siblings leave.
    Weight(f32),
}

/// A node of the tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    /// Its name: commands, toggles and the window find it by this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// What kind of node it is.
    #[serde(rename = "type")]
    pub kind: NodeKind,
    /// A row's, column's or stack's children.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Self>,
    /// A block's content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    /// The room it takes along its parent's axis; a weight of 1 by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,
    /// The least room it takes, in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f32>,
    /// The most room it takes, in points.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f32>,
    /// Whether it is left out, its room given to its siblings.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// A stack's child shown, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
    /// Painted behind it: a colour by name or `#rrggbb[aa]`, or
    /// `{"gradient": [from, to], "angle": deg}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Value>,
    /// A line round it: `{"width": 1, "color": "divider"}`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border: Option<Border>,
    /// Its corners' radius.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f32>,
    /// Room inside its edge: `8` or `[vertical, horizontal]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub padding: Option<Padding>,
    /// Room between a row's or column's children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gap: Option<f32>,
}

/// A line round a node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Border {
    /// Its width in points.
    #[serde(default = "one")]
    pub width: f32,
    /// Its colour, by name or `#rrggbb[aa]`.
    pub color: String,
}

const fn one() -> f32 {
    1.0
}

/// Room inside a node's edge.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Padding {
    /// The same on every side.
    All(f32),
    /// Top and bottom, then left and right.
    Axes([f32; 2]),
}

impl Padding {
    /// `(vertical, horizontal)`.
    pub const fn axes(self) -> (f32, f32) {
        match self {
            Self::All(all) => (all, all),
            Self::Axes([vertical, horizontal]) => (vertical, horizontal),
        }
    }
}

impl Node {
    /// A block of `content`.
    pub const fn block(content: Content) -> Self {
        Self {
            id: None,
            kind: NodeKind::Block,
            children: Vec::new(),
            content: Some(content),
            size: None,
            min: None,
            max: None,
            hidden: false,
            active: None,
            background: None,
            border: None,
            radius: None,
            padding: None,
            gap: None,
        }
    }

    /// A row or column of `children`.
    pub fn line(kind: NodeKind, children: Vec<Self>) -> Self {
        Self {
            kind,
            children,
            content: None,
            ..Self::block(Content::Canvas)
        }
    }

    /// The node with the id `id`.
    #[must_use]
    pub fn with_id(mut self, id: &str) -> Self {
        self.id = Some(id.to_owned());
        self
    }

    /// The node taking exactly `px` points.
    #[must_use]
    pub const fn px(mut self, px: f32) -> Self {
        self.size = Some(Size::Px(px));
        self
    }

    /// Whether its id is `id`.
    pub fn is(&self, id: &str) -> bool {
        self.id.as_deref() == Some(id)
    }

    /// It and every node under it, parents first.
    pub fn walk(&self) -> Vec<&Self> {
        let mut out = vec![self];
        let mut at = 0;
        while at < out.len() {
            let node = out[at];
            out.extend(node.children.iter());
            at += 1;
        }
        out
    }

    /// The node under it (or it) with the id `id`.
    pub fn find(&self, id: &str) -> Option<&Self> {
        self.walk().into_iter().find(|node| node.is(id))
    }

    /// The node under it (or it) with the id `id`, to change.
    pub fn find_mut(&mut self, id: &str) -> Option<&mut Self> {
        if self.is(id) {
            return Some(self);
        }
        self.children
            .iter_mut()
            .find_map(|child| child.find_mut(id))
    }

    /// The child a stack shows: the one `active` names, else the first.
    pub fn shown_child(&self) -> Option<&Self> {
        let named = (self.active.as_deref())
            .and_then(|active| self.children.iter().find(|child| child.is(active)));
        named.or_else(|| self.children.first())
    }

    /// The mod a block shows, and its props.
    pub fn mod_content(&self) -> Option<(&str, &Map<String, Value>)> {
        match &self.content {
            Some(Content::Mod { name, props }) => Some((name, props)),
            _ => None,
        }
    }

    /// Whether its content's `props.hidden` lists `id`.
    pub(crate) fn lists_hidden(&self, id: &str) -> bool {
        self.mod_content()
            .is_some_and(|(_, props)| lists(props, id))
    }
}

/// Whether `props.hidden` lists `id`.
pub(crate) fn lists(props: &Map<String, Value>, id: &str) -> bool {
    (props.get("hidden").and_then(Value::as_array))
        .is_some_and(|ids| ids.iter().any(|listed| listed.as_str() == Some(id)))
}
