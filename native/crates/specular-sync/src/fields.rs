//! An item as the flat field map it is stored as in the shared document,
//! and back.
//!
//! An entity is stored as its `.canvas` node, an edge and an annotation as
//! their `.canvas` objects. Each top-level key is one field, and so is each
//! key of a node's `specular` extension object, as `specular/<key>`. Two
//! peers that change different fields of one item both keep their change;
//! two that change the same field leave the later write.

use serde_json::Value;
use specular_doc::{Annotation, Edge, Entity, JsonMap};
use yrs::Any;

/// The field that holds an item's order key. Not a `.canvas` key.
pub(crate) const ORDER: &str = "~z";

const EXTENSION: &str = "specular";
const EXTENSION_PREFIX: &str = "specular/";

/// A flat field map: field name to value.
pub(crate) type Fields = Vec<(String, Any)>;

pub(crate) fn entity(entity: &Entity) -> Option<Fields> {
    match entity.to_exact_node() {
        Ok(node) => Some(flatten(node)),
        Err(error) => {
            tracing::warn!("entity {} not synced: {error}", entity.id);
            None
        }
    }
}

pub(crate) fn edge(edge: &Edge) -> Option<Fields> {
    object(edge).map(flatten)
}

pub(crate) fn annotation(annotation: &Annotation) -> Option<Fields> {
    object(annotation).map(flatten)
}

pub(crate) fn to_entity(id: &str, fields: Fields) -> Option<Entity> {
    match Entity::from_node(unflatten(id, fields)) {
        Ok(entity) => Some(entity),
        Err(node) => {
            tracing::warn!("a peer's node {id} could not be read: {node:?}");
            None
        }
    }
}

pub(crate) fn to_edge(id: &str, fields: Fields) -> Option<Edge> {
    from_object(id, fields)
}

pub(crate) fn to_annotation(id: &str, fields: Fields) -> Option<Annotation> {
    from_object(id, fields)
}

fn object<T: serde::Serialize>(item: &T) -> Option<JsonMap> {
    match serde_json::to_value(item) {
        Ok(Value::Object(map)) => Some(map),
        Ok(_) | Err(_) => None,
    }
}

fn from_object<T: serde::de::DeserializeOwned>(id: &str, fields: Fields) -> Option<T> {
    match serde_json::from_value(Value::Object(unflatten(id, fields))) {
        Ok(item) => Some(item),
        Err(error) => {
            tracing::warn!("a peer's item {id} could not be read: {error}");
            None
        }
    }
}

fn flatten(mut object: JsonMap) -> Fields {
    object.remove("id");
    let mut fields = Fields::new();
    for (key, value) in object {
        match (key.as_str(), value) {
            (EXTENSION, Value::Object(extension)) => {
                for (key, value) in extension {
                    fields.push((format!("{EXTENSION_PREFIX}{key}"), to_any(value)));
                }
            }
            (_, value) => fields.push((key, to_any(value))),
        }
    }
    fields
}

fn unflatten(id: &str, fields: Fields) -> JsonMap {
    let mut object = JsonMap::new();
    object.insert("id".to_owned(), Value::String(id.to_owned()));
    let mut extension = JsonMap::new();
    for (key, value) in fields {
        if key == ORDER {
            continue;
        }
        match key.strip_prefix(EXTENSION_PREFIX) {
            Some(key) => extension.insert(key.to_owned(), from_any(&value)),
            None => object.insert(key, from_any(&value)),
        };
    }
    if !extension.is_empty() {
        object.insert(EXTENSION.to_owned(), Value::Object(extension));
    }
    object
}

pub(crate) fn to_any(value: Value) -> Any {
    serde_json::from_value(value).unwrap_or(Any::Null)
}

fn from_any(any: &Any) -> Value {
    serde_json::to_value(any).unwrap_or(Value::Null)
}
