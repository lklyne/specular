//! JSON across the host module boundary. A value crossing into or out of a
//! host module is null, a bool, a number, a string, an array or an object,
//! which is what `serde_json` holds.

use gpui_shell::HostValue;
use serde_json::{Map, Number, Value};

/// `value` as a script sees it.
pub fn to_host(value: &Value) -> HostValue {
    match value {
        Value::Null => HostValue::Null,
        Value::Bool(flag) => HostValue::Bool(*flag),
        Value::Number(number) => HostValue::Number(number.as_f64().unwrap_or(0.0)),
        Value::String(text) => HostValue::Str(text.clone()),
        Value::Array(items) => HostValue::Array(items.iter().map(to_host).collect()),
        Value::Object(fields) => HostValue::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), to_host(value)))
                .collect(),
        ),
    }
}

/// A script's value as JSON. A number that is not finite becomes null, as
/// `JSON.stringify` makes it.
pub fn from_host(value: &HostValue) -> Value {
    match value {
        HostValue::Null => Value::Null,
        HostValue::Bool(flag) => Value::Bool(*flag),
        HostValue::Number(number) => number_value(*number),
        HostValue::Str(text) => Value::String(text.clone()),
        HostValue::Array(items) => Value::Array(items.iter().map(from_host).collect()),
        HostValue::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), from_host(value)))
                .collect::<Map<_, _>>(),
        ),
    }
}

fn number_value(number: f64) -> Value {
    if number.fract() == 0.0 && number.abs() < 9_007_199_254_740_992.0 {
        return Value::Number(Number::from(number as i64));
    }
    Number::from_f64(number).map_or(Value::Null, Value::Number)
}
