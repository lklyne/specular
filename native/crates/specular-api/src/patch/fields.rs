//! Which fields of a patch item each kind takes, and where each sits in the
//! kind's `.canvas` node. An item is turned into a node and read with the
//! file reader, so the API and a saved file cannot disagree about a field.

use serde_json::{Value, json};
use specular_doc::{JsonMap, Kind};

/// The six kinds, as a patch item's `kind` names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Sort {
    Page,
    Text,
    File,
    Group,
    Drawing,
    Shape,
}

impl Sort {
    /// The kind `name` names. `note` is `add note`'s word for a text.
    pub(super) fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "page" => Self::Page,
            "text" | "note" => Self::Text,
            "file" => Self::File,
            "group" => Self::Group,
            "drawing" => Self::Drawing,
            "shape" => Self::Shape,
            _ => return None,
        })
    }

    pub(super) const fn of(kind: &Kind) -> Self {
        match kind {
            Kind::Page(_) => Self::Page,
            Kind::Text(_) => Self::Text,
            Kind::File(_) => Self::File,
            Kind::Group(_) => Self::Group,
            Kind::Drawing(_) => Self::Drawing,
            Kind::Shape(_) => Self::Shape,
        }
    }

    /// The kind's name, which is also the prefix of the ids it is given.
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Text => "text",
            Self::File => "file",
            Self::Group => "group",
            Self::Drawing => "drawing",
            Self::Shape => "shape",
        }
    }

    /// The node `type` the kind is saved as.
    pub(super) const fn node_type(self) -> &'static str {
        match self {
            Self::Page => "link",
            Self::Text => "text",
            Self::File => "file",
            Self::Group => "group",
            Self::Drawing => "drawing",
            Self::Shape => "shape",
        }
    }

    /// The size a new one has when the item gives none.
    pub(super) const fn default_size(self) -> (f64, f64) {
        match self {
            Self::Page => (1280.0, 800.0),
            Self::Text | Self::Group | Self::Drawing => (200.0, 200.0),
            Self::File => (400.0, 300.0),
            Self::Shape => (160.0, 160.0),
        }
    }

    /// The fields the kind takes besides its position and size.
    const fn fields(self) -> &'static [Field] {
        match self {
            Self::Page => {
                const {
                    &[
                        top("url"),
                        top("presetIndex"),
                        renamed("name", "label"),
                        top("label"),
                        top("colorScheme"),
                    ]
                }
            }
            Self::Text => {
                const {
                    &[
                        top("text"),
                        COLOR,
                        ext("textStyle"),
                        ext("widthMode"),
                        ext("textSize"),
                        ext("textFont"),
                        ext("label"),
                    ]
                }
            }
            Self::File => {
                const {
                    &[
                        top("file"),
                        top("subpath"),
                        top("objectFit"),
                        top("presetIndex"),
                        ext("label"),
                    ]
                }
            }
            // `text` renames a group, and an explicit `label` wins.
            Self::Group => const { &[renamed("text", "label"), top("label"), COLOR] },
            Self::Drawing => const { &[top("strokes"), top("label")] },
            Self::Shape => {
                const {
                    &[
                        top("shapeKind"),
                        top("text"),
                        COLOR,
                        top("strokeWidth"),
                        top("borderStyle"),
                        top("borderColor"),
                        top("theme"),
                        top("label"),
                        ext("textSize"),
                        ext("fillStyle"),
                        ext("textAlign"),
                        ext("textVerticalAlign"),
                    ]
                }
            }
        }
    }
}

/// One patchable field.
struct Field {
    /// Its name in a patch item.
    item: &'static str,
    /// Its name in the node.
    node: &'static str,
    place: Place,
}

enum Place {
    /// At the node's top level.
    Top,
    /// In the node's `specular` object.
    Ext,
    /// `color`, which each kind stores its own way.
    Color,
}

const COLOR: Field = Field {
    item: "color",
    node: "color",
    place: Place::Color,
};

const fn top(name: &'static str) -> Field {
    renamed(name, name)
}

const fn renamed(item: &'static str, node: &'static str) -> Field {
    Field {
        item,
        node,
        place: Place::Top,
    }
}

const fn ext(name: &'static str) -> Field {
    Field {
        item: name,
        node: name,
        place: Place::Ext,
    }
}

fn specular(node: &mut JsonMap) -> Option<&mut JsonMap> {
    node.entry("specular")
        .or_insert_with(|| json!({}))
        .as_object_mut()
}

/// Copies the fields `item` gives onto `node`. A field the kind does not
/// take is left out, and so is a `null`.
pub(super) fn overlay(node: &mut JsonMap, item: &JsonMap, sort: Sort) {
    for field in sort.fields() {
        let Some(value) = item.get(field.item).filter(|value| !value.is_null()) else {
            continue;
        };
        match field.place {
            Place::Top => {
                node.insert(field.node.to_owned(), value.clone());
            }
            Place::Ext => {
                if let Some(ext) = specular(node) {
                    ext.insert(field.node.to_owned(), value.clone());
                }
            }
            Place::Color => put_color(node, value, sort),
        }
    }
    for (from, to) in [("canvasX", "x"), ("canvasY", "y")] {
        if let Some(value) = item.get(from).filter(|value| value.is_number()) {
            node.insert(to.to_owned(), value.clone());
        }
    }
    // A page is sized by its preset.
    if sort != Sort::Page {
        for key in ["width", "height"] {
            if let Some(value) = item.get(key).filter(|value| value.is_number()) {
                node.insert(key.to_owned(), value.clone());
            }
        }
    }
}

/// A group keeps its color under two names. A text or shape stores the
/// theme-following neutral as the red preset plus a role, so tools that
/// ignore `specular` still read a valid color.
fn put_color(node: &mut JsonMap, value: &Value, sort: Sort) {
    if sort == Sort::Group {
        node.insert("color".to_owned(), value.clone());
        node.insert("groupColor".to_owned(), value.clone());
        return;
    }
    let neutral = value.as_str() == Some("neutral");
    let stored = if neutral { json!("1") } else { value.clone() };
    node.insert("color".to_owned(), stored);
    if let Some(ext) = specular(node) {
        if neutral {
            ext.insert("colorRole".to_owned(), json!("neutral"));
        } else {
            ext.shift_remove("colorRole");
        }
    }
}

/// The first field `item` gave that the reader could not type and so left
/// in the entity's unmodeled fields.
pub(super) fn misread<'f>(extra: &JsonMap, item: &JsonMap, sort: Sort) -> Option<&'f str> {
    let ext = extra.get("specular").and_then(Value::as_object);
    (sort.fields().iter())
        .filter(|field| item.get(field.item).is_some_and(|value| !value.is_null()))
        .find(|field| match field.place {
            Place::Top | Place::Color => extra.contains_key(field.node),
            Place::Ext => ext.is_some_and(|ext| ext.contains_key(field.node)),
        })
        .map(|field| field.item)
}
