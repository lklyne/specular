//! The state keys mods read for their items and tools (`call` is in
//! [`commands`](super::commands)).
//!
//! | Key | What it holds |
//! |---|---|
//! | `canvas/mod_items` | the active canvas's mod items: `[{id, mod, width, height, props}]` |
//! | `tool` | the armed mod tool's stroke, `{tool, number, phase, points}`, or `null` |
//!
//! An item's position is left out of `canvas/mod_items`: its display list is
//! in item space, so a move needs nothing from its mod and wakes no script.

use serde_json::{Value, json};
use specular_doc::Kind;

use crate::App;

/// The `canvas/mod_items` key.
pub(crate) fn mod_items_json(app: &App) -> Value {
    let items = app.document.entities().filter_map(|entity| {
        let Kind::Mod(item) = &entity.kind else {
            return None;
        };
        Some(json!({
            "id": entity.id.as_str(),
            "mod": item.name,
            "width": entity.rect.width,
            "height": entity.rect.height,
            "props": item.props,
        }))
    });
    Value::Array(items.collect())
}

/// The `tool` key.
pub(crate) fn tool_json(app: &App) -> Value {
    serde_json::to_value(&app.mods.stroke).unwrap_or(Value::Null)
}
