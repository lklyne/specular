//! The JSON both ends of the API read and write, typed once: the routes
//! build these and the `specular` CLI reads them back.
//!
//! Most routes read their request as loose JSON and refuse what they cannot
//! use, in the Electron app's words. What is here is what the CLI itself
//! decodes, plus the checks it runs before it sends anything.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::placement::layout;
use crate::presets;

/// What the discovery file holds: where the server is and the secret every
/// request must carry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Discovery {
    /// The port the server listens on, on 127.0.0.1.
    pub port: u16,
    /// The value of the [`SECRET_HEADER`](crate::SECRET_HEADER).
    pub secret: String,
    /// The API's [`VERSION`](crate::VERSION).
    pub version: String,
}

/// The answer to `POST /canvas/apply`: the ids each part of the patch
/// touched.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Applied {
    /// The entities made, in the order the patch listed them.
    pub created: Vec<String>,
    /// The entities changed.
    pub updated: Vec<String>,
    /// What was removed. An id that named nothing is left out.
    pub deleted: Vec<String>,
    /// The edges made or changed.
    pub edges: Vec<String>,
}

/// Where one item goes: its own top-left on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    /// The item's `x`.
    pub canvas_x: f64,
    /// The item's `y`.
    pub canvas_y: f64,
}

/// The answer to `POST /layout/batch-placement` and
/// `POST /layout/apply-directive`: a position for each item asked about.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Placed {
    /// One per item, in order.
    pub positions: Vec<Position>,
    /// For a directive, the kind of each item that named an existing
    /// entity, and `null` for each new one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<Option<String>>>,
}

/// The room a new item of a patch takes on the canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Footprint {
    /// Across.
    pub width: f64,
    /// Down.
    pub height: f64,
}

/// The room the create `item` will take: a page its preset's size turned
/// to its orientation, anything else the size it names, else 200 square.
pub fn footprint(item: &Value) -> Footprint {
    if item.get("kind").and_then(Value::as_str) == Some("page") {
        let index =
            (item.get("presetIndex").and_then(Value::as_u64)).unwrap_or(u64::from(presets::LAPTOP));
        let size = presets::size(index).unwrap_or((1280.0, 800.0));
        let (width, height) =
            presets::oriented(size, item.get("orientation").and_then(Value::as_str));
        return Footprint { width, height };
    }
    let side = |key: &str| {
        (item.get(key).and_then(Value::as_f64))
            .filter(|side| *side != 0.0)
            .unwrap_or(200.0)
    };
    Footprint {
        width: side("width"),
        height: side("height"),
    }
}

/// Checks a patch's `layout` directive, returning the first problem in the
/// words the routes use.
pub fn check_layout(directive: &Value) -> Result<(), String> {
    layout::validate(directive).map(|_| ())
}
