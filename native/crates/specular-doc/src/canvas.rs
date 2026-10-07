//! The `.canvas` reader and writer: JSON Canvas v1.0 plus Specular's
//! extensions, as `docs/file-formats.md` describes.
//!
//! Loading never drops data. A field this crate does not model stays in the
//! `extra` of the item it sat on. A whole node, edge or annotation that
//! cannot be typed (an unknown node `type`, a missing required field, a
//! duplicate id) stays as raw JSON in [`Document::extra`] under the key of
//! the array it came from, and is written back after the typed items.
//!
//! Saving is canonical rather than byte-preserving: it writes what the
//! Electron writer would, so a file either app wrote loads and saves to the
//! same JSON value.

mod fields;
mod read;
#[cfg(test)]
mod tests;
mod write;

use std::collections::{HashMap, HashSet};

use serde_json::Value;

use self::fields::{fill, read_loose, take, tidy_numbers, write_loose};
use crate::{Annotation, Command, CommandError, Document, Edge, Entity, JsonMap};

/// Why a `.canvas` file could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum CanvasError {
    /// The text is not JSON.
    #[error("the canvas is not valid JSON")]
    Parse(#[source] serde_json::Error),
    /// The top-level value is not a JSON object.
    #[error("the canvas is not a JSON object")]
    NotAnObject,
    /// `nodes`, `edges` or `annotations` is present but is not an array.
    #[error("the canvas's `{0}` is not an array")]
    NotAnArray(&'static str),
    /// The loaded items could not be placed in a document.
    #[error("the canvas could not be loaded")]
    Command(#[from] CommandError),
    /// A value could not be written as JSON.
    #[error("the canvas could not be written as JSON")]
    Encode(#[source] serde_json::Error),
}

const NODES: &str = "nodes";
const EDGES: &str = "edges";
const ANNOTATIONS: &str = "annotations";
const SPECULAR: &str = "specular";
const ENTITY_ORDER: &str = "entityOrder";

/// An entity or edge waiting for its slot in the stack order.
enum Item {
    Entity(Box<Entity>),
    Edge(Box<Edge>),
}

impl Document {
    /// Reads a `.canvas` file's text.
    pub fn from_canvas_str(json: &str) -> Result<Self, CanvasError> {
        Self::from_canvas_value(serde_json::from_str(json).map_err(CanvasError::Parse)?)
    }

    /// Reads a `.canvas` file's parsed JSON.
    pub fn from_canvas_value(value: Value) -> Result<Self, CanvasError> {
        let Value::Object(mut top) = value else {
            return Err(CanvasError::NotAnObject);
        };
        let nodes = take_array(&mut top, NODES)?;
        let edges = take_array(&mut top, EDGES)?;
        let annotations = take_array(&mut top, ANNOTATIONS)?;

        // Entities and edges share one id namespace and one stack order.
        let mut items = HashMap::new();
        let mut file_order = Vec::new();
        let mut stray_nodes = Vec::new();
        for node in nodes {
            match node {
                Value::Object(node) if !items.contains_key(id_of(&node)) => {
                    match read::read_node(node) {
                        Ok(entity) => {
                            file_order.push(entity.id.as_str().to_owned());
                            items.insert(
                                entity.id.as_str().to_owned(),
                                Item::Entity(Box::new(entity)),
                            );
                        }
                        Err(node) => stray_nodes.push(Value::Object(node)),
                    }
                }
                stray => stray_nodes.push(stray),
            }
        }
        let mut stray_edges = Vec::new();
        for raw in edges {
            match read_loose::<Edge>(&raw) {
                Some(edge) if !items.contains_key(edge.id.as_str()) => {
                    file_order.push(edge.id.as_str().to_owned());
                    items.insert(edge.id.as_str().to_owned(), Item::Edge(Box::new(edge)));
                }
                Some(_) | None => stray_edges.push(raw),
            }
        }

        let mut specular: JsonMap = take(&mut top, SPECULAR).unwrap_or_default();
        let saved_order: Vec<String> = take(&mut specular, ENTITY_ORDER).unwrap_or_default();

        // The saved order first, then anything it does not list, in file
        // order. Ids it lists that no longer exist are skipped.
        let mut document = Self::new();
        for id in saved_order.iter().chain(&file_order) {
            let command = match items.remove(id) {
                Some(Item::Entity(entity)) => Command::InsertEntity {
                    entity,
                    at: document.stack_len(),
                },
                Some(Item::Edge(edge)) => Command::InsertEdge {
                    edge,
                    at: document.stack_len(),
                },
                None => continue,
            };
            document.apply(command)?;
        }

        let mut stray_annotations = Vec::new();
        let mut seen = HashSet::new();
        for raw in annotations {
            match read_loose::<Annotation>(&raw) {
                Some(annotation) if seen.insert(annotation.id.clone()) => {
                    document.apply(Command::InsertAnnotation {
                        annotation: Box::new(annotation),
                        at: document.annotations().len(),
                    })?;
                }
                Some(_) | None => stray_annotations.push(raw),
            }
        }

        for (key, stray) in [
            (NODES, stray_nodes),
            (EDGES, stray_edges),
            (ANNOTATIONS, stray_annotations),
        ] {
            if !stray.is_empty() {
                top.insert(key.to_owned(), Value::Array(stray));
            }
        }
        if !specular.is_empty() {
            top.insert(SPECULAR.to_owned(), Value::Object(specular));
        }
        *document.extra_mut() = top;
        Ok(document)
    }

    /// The document as a `.canvas` file's JSON.
    pub fn to_canvas_value(&self) -> Result<Value, CanvasError> {
        let extra = self.extra();
        let mut top = JsonMap::new();

        let mut nodes = Vec::new();
        for entity in self.entities() {
            nodes.push(Value::Object(write::write_node(entity)?));
        }
        nodes.extend(stray(extra, NODES));
        top.insert(NODES.to_owned(), Value::Array(nodes));

        let mut edges = Vec::new();
        for edge in self.edges() {
            edges.push(write_loose(edge)?);
        }
        edges.extend(stray(extra, EDGES));
        top.insert(EDGES.to_owned(), Value::Array(edges));

        let mut specular = JsonMap::new();
        if !self.order().is_empty() {
            let order = self.order().iter().map(|item| Value::from(item.as_str()));
            specular.insert(ENTITY_ORDER.to_owned(), order.collect());
        }
        if let Some(Value::Object(leftover)) = extra.get(SPECULAR) {
            fill(&mut specular, leftover);
        }
        if !specular.is_empty() {
            top.insert(SPECULAR.to_owned(), Value::Object(specular));
        }

        let mut annotations = Vec::new();
        for annotation in self.annotations() {
            annotations.push(write_loose(annotation)?);
        }
        annotations.extend(stray(extra, ANNOTATIONS));
        if !annotations.is_empty() {
            top.insert(ANNOTATIONS.to_owned(), Value::Array(annotations));
        }

        fill(&mut top, extra);
        let mut value = Value::Object(top);
        tidy_numbers(&mut value, true);
        Ok(value)
    }

    /// The document as a `.canvas` file's text, indented two spaces with no
    /// trailing newline, as the Electron app writes it.
    pub fn to_canvas_string(&self) -> Result<String, CanvasError> {
        Ok(format!("{:#}", self.to_canvas_value()?))
    }
}

/// Removes a top-level array. A missing one reads as empty.
fn take_array(top: &mut JsonMap, key: &'static str) -> Result<Vec<Value>, CanvasError> {
    match top.shift_remove(key) {
        Some(Value::Array(items)) => Ok(items),
        None => Ok(Vec::new()),
        Some(_) => Err(CanvasError::NotAnArray(key)),
    }
}

/// A raw node's id, or `""` when it has none; such a node is refused by the
/// reader and kept raw whatever this returns.
fn id_of(node: &JsonMap) -> &str {
    node.get("id").and_then(Value::as_str).unwrap_or_default()
}

/// The raw items a load could not type, kept under their array's key.
fn stray<'a>(extra: &'a JsonMap, key: &str) -> impl Iterator<Item = Value> + 'a {
    extra
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .cloned()
}
