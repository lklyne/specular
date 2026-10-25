//! Every canvas write is one patch through `POST /canvas/apply` (ADR 0019).
//! Before it is sent, each new item that names no position is given one:
//! by the patch's `layout` directive when it has one, else in a free spot
//! the app finds for the whole batch.

use std::io::Write as _;

use anyhow::Context as _;
use serde_json::{Map, Value, json};
use specular_api::{Applied, Placed, footprint};

use crate::client::Client;
use crate::js::num;

/// How a batch of new items with no position is laid out, when there is
/// no directive: `upsert`'s `--layout` and `--gap`.
#[derive(Debug, Clone, Default)]
pub(crate) struct Batch {
    pub(crate) layout: Option<String>,
    pub(crate) gap: Option<f64>,
}

fn placed(answer: Value) -> anyhow::Result<Placed> {
    serde_json::from_value(answer).context("the app's placement answer was not understood")
}

fn set_position(item: &mut Value, x: f64, y: f64) {
    item["canvasX"] = num(x);
    item["canvasY"] = num(y);
}

/// Lays out every item, new or existing, by `directive`.
fn by_directive(client: &Client, items: &mut [Value], directive: &Value) -> anyhow::Result<()> {
    let asked: Vec<Value> = (items.iter())
        .map(|item| {
            let Some(id) = item.get("id") else {
                let size = footprint(item);
                return json!({ "width": num(size.width), "height": num(size.height) });
            };
            let mut out = json!({ "id": id });
            for side in ["width", "height"] {
                if let Some(value) = item.get(side) {
                    out[side] = value.as_f64().map_or(Value::Null, num);
                }
            }
            out
        })
        .collect();
    let answer = client.post(
        "/layout/apply-directive",
        &json!({ "layout": directive, "items": asked }),
    )?;
    let placed = placed(answer)?;
    let kinds = placed.kinds.unwrap_or_default();
    let mut warnings = Vec::new();
    for (index, (item, at)) in items.iter_mut().zip(&placed.positions).enumerate() {
        if item.get("canvasX").is_some() || item.get("canvasY").is_some() {
            warnings.push(format!(
                "item[{index}]: canvasX/canvasY ignored under layout directive"
            ));
        }
        set_position(item, at.canvas_x, at.canvas_y);
        if item.get("id").is_some()
            && item.get("kind").is_none()
            && let Some(Some(kind)) = kinds.get(index)
        {
            item["kind"] = json!(kind);
        }
    }
    if !warnings.is_empty() {
        let _ = writeln!(
            std::io::stderr(),
            "upsert layout directive: {}",
            warnings.join("; ")
        );
    }
    Ok(())
}

/// Puts the new items that have no position together in one free spot.
fn in_free_spot(client: &Client, items: &mut [Value], batch: &Batch) -> anyhow::Result<()> {
    let mut waiting: Vec<&mut Value> = (items.iter_mut())
        .filter(|item| {
            item.get("id").is_none()
                && (item.get("canvasX").is_none() || item.get("canvasY").is_none())
        })
        .collect();
    if waiting.is_empty() {
        return Ok(());
    }
    let sizes: Vec<Value> = (waiting.iter())
        .map(|item| {
            let size = footprint(item);
            json!({ "width": num(size.width), "height": num(size.height) })
        })
        .collect();
    let mut body = json!({ "items": sizes, "anchor": "selection_or_empty_region" });
    if let Some(layout) = &batch.layout {
        body["layout"] = json!(layout);
    }
    if let Some(gap) = batch.gap {
        body["gap"] = num(gap);
    }
    let placed = placed(client.post("/layout/batch-placement", &body)?)?;
    for (item, at) in waiting.iter_mut().zip(&placed.positions) {
        set_position(item, at.canvas_x, at.canvas_y);
    }
    Ok(())
}

/// Sends `patch` (`entities`, `edges`, `delete` and a `layout` directive)
/// as one transaction, after placing its new items.
pub(crate) fn apply(client: &Client, patch: &Value, batch: &Batch) -> anyhow::Result<Applied> {
    let mut body = Map::new();
    match patch.get("entities") {
        Some(Value::Array(items)) => {
            let mut entities = items.clone();
            if let Some(directive) = patch.get("layout").filter(|layout| !layout.is_null()) {
                by_directive(client, &mut entities, directive)?;
            }
            in_free_spot(client, &mut entities, batch)?;
            body.insert("entities".to_owned(), Value::Array(entities));
        }
        // Anything else goes as it came, for the app to refuse in its words.
        Some(other) if !other.is_null() => {
            body.insert("entities".to_owned(), other.clone());
        }
        _ => {
            body.insert("entities".to_owned(), Value::Array(Vec::new()));
        }
    }
    for key in ["edges", "delete"] {
        if let Some(value) = patch.get(key).filter(|value| !value.is_null()) {
            body.insert(key.to_owned(), value.clone());
        }
    }
    let answer = client.post("/canvas/apply", &Value::Object(body))?;
    serde_json::from_value(answer).context("the app's apply answer was not understood")
}
