//! Reading and comparing the canvas as a save writes it, as JSON.
//!
//! A path is a JSON pointer whose array steps may also be an item's `id`
//! or a negative index from the end: `/nodes/s1/color`,
//! `/annotations/-1/text`. Numbers compare by value, so `200` is `200.0`.

use std::fmt::Write as _;

use serde_json::Value;

/// The value at `path` in `root`, if there is one.
pub(super) fn at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut here = root;
    for step in path.split('/').skip(1) {
        here = match here {
            Value::Object(map) => map.get(step)?,
            Value::Array(items) => item(items, step)?,
            _ => return None,
        };
    }
    Some(here)
}

fn item<'a>(items: &'a [Value], step: &str) -> Option<&'a Value> {
    if let Some(found) = items
        .iter()
        .find(|item| item.get("id").and_then(Value::as_str) == Some(step))
    {
        return Some(found);
    }
    let index: i64 = step.parse().ok()?;
    let index = if index < 0 {
        items.len().checked_sub(index.unsigned_abs() as usize)?
    } else {
        index as usize
    };
    items.get(index)
}

/// `path` as [`differences`] names it: an array step that is an index is
/// written as the item's `id`, when it has one in `root`.
pub(super) fn canonical(root: &Value, path: &str) -> String {
    let mut here = Some(root);
    let mut out = String::new();
    for step in path.split('/').skip(1) {
        let next = match here {
            Some(Value::Object(map)) => map.get(step),
            Some(Value::Array(items)) => item(items, step),
            _ => None,
        };
        let named = next
            .filter(|_| matches!(here, Some(Value::Array(_))))
            .and_then(|item| item.get("id"))
            .and_then(Value::as_str);
        out.push('/');
        out.push_str(named.unwrap_or(step));
        here = next;
    }
    out
}

/// Whether `a` and `b` are the same JSON, numbers compared by value.
pub(super) fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(x, y)| same(x, y))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(key, x)| y.get(key).is_some_and(|y| same(x, y)))
        }
        _ => a == b,
    }
}

/// One place where two canvases differ: its path and both values.
pub(super) type Difference = (String, String, String);

/// Where `a` and `b` differ, leaving out `/appState` (the camera and the
/// open tabs are a way of looking, not the document). Arrays of objects
/// with ids in the same order are compared item by item under their ids.
pub(super) fn differences(a: &Value, b: &Value) -> Vec<Difference> {
    let mut out = Vec::new();
    walk(a, b, String::new(), &mut out);
    out.retain(|(path, _, _)| !path.starts_with("/appState"));
    out
}

fn short(value: Option<&Value>) -> String {
    let text = value.map_or_else(|| "<absent>".to_owned(), Value::to_string);
    match text.char_indices().nth(70) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text,
    }
}

fn walk(a: &Value, b: &Value, path: String, out: &mut Vec<Difference>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let here = format!("{path}/{key}");
                match (x.get(key), y.get(key)) {
                    (Some(x), Some(y)) => walk(x, y, here, out),
                    (x, y) => out.push((here, short(x), short(y))),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            let id = |item: &Value| item.get("id").and_then(Value::as_str).map(str::to_owned);
            let ids_x: Option<Vec<String>> = x.iter().map(id).collect();
            let ids_y: Option<Vec<String>> = y.iter().map(id).collect();
            match (ids_x, ids_y) {
                (Some(ids_x), Some(ids_y)) if ids_x == ids_y => {
                    for ((x, y), id) in x.iter().zip(y).zip(&ids_x) {
                        walk(x, y, format!("{path}/{id}"), out);
                    }
                }
                (Some(_), Some(_)) => out.push((
                    path,
                    format!("{} items", x.len()),
                    format!("{} items, or another order", y.len()),
                )),
                _ if x.len() == y.len() => {
                    for (index, (x, y)) in x.iter().zip(y).enumerate() {
                        walk(x, y, format!("{path}/{index}"), out);
                    }
                }
                _ => out.push((
                    path,
                    format!("{} items", x.len()),
                    format!("{} items", y.len()),
                )),
            }
        }
        _ if same(a, b) => {}
        _ => out.push((path, short(Some(a)), short(Some(b)))),
    }
}

/// `found` as lines under a heading, for a failure message.
pub(super) fn listed(found: &[Difference]) -> String {
    let mut out = String::new();
    for (path, was, now) in found {
        let _ = write!(out, "\n    {path}: {was} -> {now}");
    }
    out
}
