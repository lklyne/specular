//! Numbers read from flags and written into request bodies the way the
//! routes expect them.

use std::io::{IsTerminal as _, Read as _};

use serde_json::{Value, json};

/// `text` as a number: blank is 0, anything unreadable `None`.
pub(crate) fn number(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return Some(0.0);
    }
    text.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// `text` as a number, or `fallback` when it is unreadable or 0.
pub(crate) fn number_or(text: Option<&str>, fallback: f64) -> f64 {
    text.and_then(number)
        .filter(|value| *value != 0.0)
        .unwrap_or(fallback)
}

/// `value` as JSON, a whole number written without a fraction: a route
/// that wants an index reads `9`, not `9.0`.
pub(crate) fn num(value: f64) -> Value {
    if value.fract() == 0.0 && value.abs() < 9.0e15 {
        json!(value as i64)
    } else {
        json!(value)
    }
}

/// `x,y` as its two numbers, each `None` when unreadable.
pub(crate) fn pair(text: &str) -> (Option<f64>, Option<f64>) {
    let mut parts = text.split(',');
    let x = parts.next().and_then(number);
    let y = parts.next().and_then(number);
    (x, y)
}

/// Everything on stdin.
pub(crate) fn read_stdin() -> anyhow::Result<String> {
    let mut text = String::new();
    std::io::stdin().read_to_string(&mut text)?;
    Ok(text)
}

/// Whether stdin is a terminal, where a read would wait for typing.
pub(crate) fn stdin_is_terminal() -> bool {
    std::io::stdin().is_terminal()
}
