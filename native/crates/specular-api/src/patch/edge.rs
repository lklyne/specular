//! One edge item of a patch. An id that names an edge patches it in place;
//! anything else creates one.

use serde_json::{Value, json};
use specular_doc::{Command, Edge, EdgeId, EntityId, JsonMap};

use super::Builder;
use crate::Response;

/// A patch item's field names beside the edge's own in the `.canvas` file.
const FIELDS: [(&str, &str); 12] = [
    ("fromEntityId", "fromNode"),
    ("toEntityId", "toNode"),
    ("kind", "edgeKind"),
    ("metadata", "edgeMetadata"),
    ("label", "label"),
    ("color", "color"),
    ("fromSide", "fromSide"),
    ("toSide", "toSide"),
    ("fromEnd", "fromEnd"),
    ("toEnd", "toEnd"),
    ("strokeWidth", "strokeWidth"),
    ("lineStyle", "lineStyle"),
];

pub(super) fn upsert(builder: &mut Builder<'_>, at: &str, item: &JsonMap) -> Result<(), Response> {
    let existing = (item.get("id").and_then(Value::as_str))
        .and_then(|id| builder.trial.edge(&EdgeId::from(id)));
    let (mut edge, id, creating) = if let Some(edge) = existing {
        let json =
            serde_json::to_value(edge).map_err(|error| Response::error(500, error.to_string()))?;
        (json, edge.id.as_str().to_owned(), false)
    } else {
        let id = builder.fresh_id("edge");
        (json!({ "id": id }), id, true)
    };
    for (from, to) in FIELDS {
        if let Some(value) = item.get(from).filter(|value| !value.is_null()) {
            edge[to] = value.clone();
        }
    }
    for (key, end) in [("fromNode", "fromEntityId"), ("toNode", "toEntityId")] {
        let Some(entity) = edge.get(key).and_then(Value::as_str) else {
            return Err(Response::bad_request(format!(
                "{at}: fromEntityId and toEntityId are required"
            )));
        };
        if builder.trial.entity(&EntityId::from(entity)).is_none() {
            return Err(Response::bad_request(format!(
                "{at}.{end}: unknown entity '{entity}'"
            )));
        }
    }
    let edge: Edge = serde_json::from_value(edge)
        .map_err(|error| Response::bad_request(format!("{at}: {error}")))?;
    let command = if creating {
        Command::InsertEdge {
            edge: Box::new(edge),
            at: builder.trial.stack_len(),
        }
    } else {
        Command::ReplaceEdge(Box::new(edge))
    };
    builder.push(command, at)?;
    builder.edges.push(id);
    Ok(())
}
