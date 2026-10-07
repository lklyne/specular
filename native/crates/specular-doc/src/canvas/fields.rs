//! Field-level helpers shared by the `.canvas` reader and writer.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Number, Value};

use super::CanvasError;
use crate::{Annotation, Edge, JsonMap};

/// Removes `key` from `map` and returns it typed. A value that does not fit
/// `T` (a `null`, an unknown enum string) stays in the map, which becomes
/// the item's `extra`, so the writer puts it back as it was.
pub(super) fn take<T: DeserializeOwned>(map: &mut JsonMap, key: &str) -> Option<T> {
    let value = T::deserialize(map.get(key)?).ok()?;
    map.shift_remove(key);
    Some(value)
}

/// Writes a required field.
pub(super) fn set<T: Serialize>(
    map: &mut JsonMap,
    key: &str,
    value: &T,
) -> Result<(), CanvasError> {
    let value = serde_json::to_value(value).map_err(CanvasError::Encode)?;
    map.insert(key.to_owned(), value);
    Ok(())
}

/// Adds the entries of `extra` whose keys `map` does not already hold, so a
/// typed field always wins over a leftover with the same name.
pub(super) fn fill(map: &mut JsonMap, extra: &JsonMap) {
    for (key, value) in extra {
        if !map.contains_key(key) {
            map.insert(key.clone(), value.clone());
        }
    }
}

/// A type that derives serde in its wire shape, read leniently: an optional
/// field that does not fit moves to `extra` instead of refusing the item.
pub(super) trait Loose: Serialize + DeserializeOwned + Clone {
    /// The wire names of the optional fields.
    const OPTIONAL: &'static [&'static str];

    /// The item's unmodeled fields.
    fn extra_mut(&mut self) -> &mut JsonMap;
}

impl Loose for Edge {
    const OPTIONAL: &'static [&'static str] = &[
        "fromSide",
        "toSide",
        "fromEnd",
        "toEnd",
        "color",
        "label",
        "strokeWidth",
        "lineStyle",
        "edgeKind",
        "edgeMetadata",
    ];

    fn extra_mut(&mut self) -> &mut JsonMap {
        &mut self.extra
    }
}

impl Loose for Annotation {
    const OPTIONAL: &'static [&'static str] = &["elementName", "pageAnchor", "metadata"];

    fn extra_mut(&mut self) -> &mut JsonMap {
        &mut self.extra
    }
}

/// Reads a [`Loose`] item. `None` means a required field is missing or
/// malformed, and the caller keeps the raw value instead.
pub(super) fn read_loose<T: Loose>(raw: &Value) -> Option<T> {
    let object = raw.as_object()?;
    // serde reads `null` into an `Option` as absent, which would drop the key
    // on save, so a null takes the slow path and lands in `extra`.
    let has_null = T::OPTIONAL
        .iter()
        .any(|key| object.get(*key).is_some_and(Value::is_null));
    if !has_null && let Ok(item) = T::deserialize(raw) {
        return Some(item);
    }

    let mut fit = object.clone();
    let mut optional = Vec::new();
    for key in T::OPTIONAL {
        if let Some(value) = fit.shift_remove(*key) {
            optional.push((*key, value));
        }
    }
    let mut fit = Value::Object(fit);
    T::deserialize(&fit).ok()?;

    let mut misfits = JsonMap::new();
    for (key, value) in optional {
        if !value.is_null() {
            fit[key] = value.clone();
            if T::deserialize(&fit).is_ok() {
                continue;
            }
            if let Some(object) = fit.as_object_mut() {
                object.shift_remove(key);
            }
        }
        misfits.insert(key.to_owned(), value);
    }
    let mut item = T::deserialize(&fit).ok()?;
    item.extra_mut().extend(misfits);
    Some(item)
}

/// Writes a [`Loose`] item, typed fields first.
pub(super) fn write_loose<T: Loose>(item: &T) -> Result<Value, CanvasError> {
    // Serialized in one go, a flattened `extra` would overwrite a typed field
    // of the same name.
    let mut bare = item.clone();
    let extra = std::mem::take(bare.extra_mut());
    let mut value = serde_json::to_value(&bare).map_err(CanvasError::Encode)?;
    if let Some(object) = value.as_object_mut() {
        fill(object, &extra);
    }
    Ok(value)
}

/// Rounds every float in the tree to a hundredth and writes whole floats as
/// integers, as the Electron writer does: canvas numbers are pixels, and
/// anything finer is float noise that shows up as a change in a diff.
///
/// `zoom` is a multiplier, not a pixel measure, so it keeps its precision.
pub(super) fn tidy_numbers(value: &mut Value, round: bool) {
    match value {
        Value::Number(number) if number.is_f64() => {
            if let Some(float) = number.as_f64() {
                *number = tidy(float, round).unwrap_or_else(|| number.clone());
            }
        }
        Value::Array(items) => {
            for item in items {
                tidy_numbers(item, round);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                tidy_numbers(item, key != "zoom");
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// Floats at or past 2^53 are not exact integers, so they stay floats.
const MAX_EXACT_INTEGER: f64 = 9_007_199_254_740_992.0;

fn tidy(float: f64, round: bool) -> Option<Number> {
    // `floor(x + 0.5)` rounds halves up, as JavaScript's `Math.round` does;
    // `f64::round` would send -0.125 to -0.13 where Electron writes -0.12.
    let float = if round {
        (float * 100.0 + 0.5).floor() / 100.0
    } else {
        float
    };
    if float.fract() == 0.0 && float.abs() < MAX_EXACT_INTEGER {
        Some(Number::from(float as i64))
    } else {
        Number::from_f64(float)
    }
}
