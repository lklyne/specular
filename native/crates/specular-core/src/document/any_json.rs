//! Lossless conversion between `serde_json` values and yrs [`Any`].
//!
//! yrs's own serde impls write every number as a float, which would turn a
//! saved `"presetIndex": 1` into `1.0` (a different `serde_json::Value`, and a
//! noisy diff). These conversions keep integers integral in both directions.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Number as JsonNumber, Value};
use yrs::Any;
use yrs::any::Number;

use crate::json_canvas::whole_number;

/// Converts a JSON value to a yrs [`Any`].
pub(super) fn value_to_any(value: &Value) -> Any {
    match value {
        Value::Null => Any::Null,
        Value::Bool(b) => Any::Bool(*b),
        Value::Number(n) => Any::Number(json_number_to_any(n)),
        Value::String(s) => Any::String(Arc::from(s.as_str())),
        Value::Array(items) => Any::Array(items.iter().map(value_to_any).collect()),
        Value::Object(fields) => Any::Map(Arc::new(object_to_any_map(fields))),
    }
}

/// Converts a JSON object to the field map of a yrs [`Any::Map`].
pub(super) fn object_to_any_map(fields: &Map<String, Value>) -> HashMap<String, Any> {
    fields
        .iter()
        .map(|(key, field)| (key.clone(), value_to_any(field)))
        .collect()
}

/// Converts a yrs [`Any`] back to a JSON value.
pub(super) fn any_to_value(any: &Any) -> Value {
    match any {
        Any::Null | Any::Undefined => Value::Null,
        Any::Bool(b) => Value::Bool(*b),
        Any::Number(n) => any_number_to_value(*n),
        Any::String(s) => Value::String(s.to_string()),
        Any::Buffer(bytes) => Value::Array(bytes.iter().map(|b| Value::from(*b)).collect()),
        Any::Array(items) => Value::Array(items.iter().map(any_to_value).collect()),
        Any::Map(fields) => Value::Object(any_map_to_object(fields)),
    }
}

/// Converts the fields of a yrs [`Any::Map`] back to a JSON object.
pub(super) fn any_map_to_object(fields: &HashMap<String, Any>) -> Map<String, Value> {
    fields
        .iter()
        .map(|(key, field)| (key.clone(), any_to_value(field)))
        .collect()
}

fn json_number_to_any(number: &JsonNumber) -> Number {
    match number.as_i64() {
        Some(int) => Number::Int(int),
        // Integers above i64::MAX and every fractional value.
        None => Number::Float(number.as_f64().unwrap_or(f64::NAN)),
    }
}

fn any_number_to_value(number: Number) -> Value {
    match number {
        Number::Int(int) => Value::from(int),
        Number::Float(float) => match whole_number(float) {
            Some(int) => Value::from(int),
            // JSON has no NaN/Infinity; JSON.stringify writes them as null too.
            None => JsonNumber::from_f64(float).map_or(Value::Null, Value::Number),
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn integers_survive_round_trip_as_integers() {
        let value = json!({"presetIndex": 1, "nested": [0, -3, {"deep": 7}]});
        assert_eq!(any_to_value(&value_to_any(&value)), value);
    }

    #[test]
    fn fractions_and_nulls_survive_round_trip() {
        let value = json!({"zoom": 0.411_999_999_999_999_53, "syncId": null, "ok": true});
        assert_eq!(any_to_value(&value_to_any(&value)), value);
    }

    #[test]
    fn whole_float_any_converts_to_json_integer() {
        let value = any_to_value(&Any::Number(Number::Float(100.0)));
        assert_eq!(value, json!(100));
    }

    #[test]
    fn non_finite_float_any_converts_to_null() {
        let value = any_to_value(&Any::Number(Number::Float(f64::INFINITY)));
        assert_eq!(value, Value::Null);
    }
}
