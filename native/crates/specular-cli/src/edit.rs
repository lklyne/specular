//! The verbs that write the canvas, each one patch (ADR 0019): `apply`
//! itself, `add`, `update`, `delete`, `link`, `group` and the older
//! `upsert`.

use anyhow::Context as _;
use serde_json::{Map, Value, json};
use specular_api::{Applied, check_layout};

use crate::args::Args;
use crate::client::Client;
use crate::js::{num, number, pair, read_stdin, stdin_is_terminal};
use crate::patch::{self, Batch};
use crate::{output, refuse, usage};

const APPLY_USAGE: &str = "specular apply [--tab <tab-id|tab-name>] < patch.json  (the patch is \
                           read from stdin; `--json` is upsert's flag, not apply's)";

fn entities(items: &[Value]) -> Value {
    json!({ "entities": items })
}

/// `apply < patch.json`: the one declarative door.
pub(crate) fn apply(client: &Client, _args: &Args) -> anyhow::Result<i32> {
    // Without these a bare `apply` waits on the terminal, and a mistaken
    // `--json '{...}'` reads as a bare parse error.
    if stdin_is_terminal() {
        return usage(APPLY_USAGE);
    }
    let input = read_stdin()?;
    let input = input.trim();
    if input.is_empty() {
        return usage(APPLY_USAGE);
    }
    let patch: Value = match serde_json::from_str(input) {
        Ok(patch) => patch,
        Err(error) => {
            return refuse(&format!(
                "apply: patch is not valid JSON ({error})\nusage: {APPLY_USAGE}"
            ));
        }
    };
    if let Some(layout) = patch.get("layout").filter(|layout| !layout.is_null())
        && let Err(problem) = check_layout(layout)
    {
        return refuse(&format!("apply: {problem}"));
    }
    let applied = patch::apply(client, &patch, &Batch::default())?;
    output::json(&serde_json::to_value(applied)?);
    Ok(0)
}

/// `upsert --json < items.json`: an array of items, or `{layout, items}`.
pub(crate) fn upsert(client: &Client, args: &Args) -> anyhow::Result<i32> {
    if !args.switch("json") {
        return usage("specular upsert --json < items.json");
    }
    let parsed: Value =
        serde_json::from_str(&read_stdin()?).context("upsert: items are not JSON")?;
    let (items, layout) = match parsed {
        Value::Array(items) => (items, None),
        Value::Object(mut object) if object.get("items").is_some_and(Value::is_array) => {
            let items = match object.remove("items") {
                Some(Value::Array(items)) => items,
                _ => Vec::new(),
            };
            (
                items,
                object.remove("layout").filter(|layout| !layout.is_null()),
            )
        }
        _ => return refuse("upsert: expected an array of items or { layout, items }"),
    };
    if let Some(layout) = &layout
        && let Err(problem) = check_layout(layout)
    {
        return refuse(&format!("upsert: {problem}"));
    }
    // The flags lay out a batch only when there is no directive.
    let batch = if layout.is_some() {
        Batch::default()
    } else {
        Batch {
            layout: args.flag("layout").map(str::to_owned),
            gap: args.flag("gap").and_then(number),
        }
    };
    let mut patch = entities(&items);
    if let Some(layout) = layout {
        patch["layout"] = layout;
    }
    let Applied {
        created, updated, ..
    } = patch::apply(client, &patch, &batch)?;
    output::json(&json!({ "created": created, "updated": updated }));
    Ok(0)
}

fn at_position(item: &mut Map<String, Value>, args: &Args) {
    let Some(at) = args.flag("at") else { return };
    let (x, y) = pair(at);
    if let Some(x) = x {
        item.insert("canvasX".to_owned(), num(x));
    }
    if let Some(y) = y {
        item.insert("canvasY".to_owned(), num(y));
    }
}

fn device_frame(item: &mut Map<String, Value>, args: &Args) {
    if args.switch("device-frame") {
        item.insert("showDeviceFrame".to_owned(), json!(true));
    } else if args.switch("no-device-frame") {
        item.insert("showDeviceFrame".to_owned(), json!(false));
    }
}

/// The create item `add <kind> ...` asks for, or its usage.
fn add_item(args: &Args) -> Result<Map<String, Value>, &'static str> {
    let mut item = Map::new();
    match args.nth(0) {
        Some("page") => {
            let url = args
                .nth(1)
                .ok_or("specular add page <url> [--at x,y] [--preset N]")?;
            item.insert("kind".to_owned(), json!("page"));
            item.insert("url".to_owned(), json!(url));
            let preset = args.flag("preset").map_or(Some(6.0), number);
            item.insert("presetIndex".to_owned(), preset.map_or(Value::Null, num));
            at_position(&mut item, args);
            if args.switch("landscape") {
                item.insert("orientation".to_owned(), json!("landscape"));
            }
            device_frame(&mut item, args);
        }
        Some("note") => {
            let text = args.words(1);
            if text.is_empty() {
                return Err("specular add note <text> [--at x,y] [--color N]");
            }
            // Long or structured text becomes a `.md` Document; the app
            // decides.
            item.insert("kind".to_owned(), json!("text"));
            item.insert("text".to_owned(), json!(text));
            at_position(&mut item, args);
            if let Some(color) = args.flag("color") {
                item.insert("color".to_owned(), json!(color));
            }
        }
        Some("file") => {
            let path = args.nth(1).ok_or("specular add file <path> [--at x,y]")?;
            item.insert("kind".to_owned(), json!("file"));
            item.insert("file".to_owned(), json!(path));
            at_position(&mut item, args);
            device_frame(&mut item, args);
        }
        _ => return Err("specular add <page|note|file> ..."),
    }
    Ok(item)
}

/// `add <page|note|file> ...`: the one create verb.
pub(crate) fn add(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let item = match add_item(args) {
        Ok(item) => item,
        Err(line) => return usage(line),
    };
    let applied = patch::apply(client, &entities(&[Value::Object(item)]), &Batch::default())?;
    output::json(&json!({ "created": applied.created }));
    Ok(0)
}

/// `update <id> [--at] [--size] [--preset] [--url] [--text] [--color]
/// [--gap] ...`. The kind is read from the canvas, and a flag the kind has
/// no use for does nothing.
pub(crate) fn update(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let Some(id) = args.nth(0) else {
        return usage(
            "specular update <id> [--at x,y] [--size w,h] [--preset N] [--text T] [--color C] \
             [--url U] [--gap N]",
        );
    };
    let mut item = Map::new();
    item.insert("id".to_owned(), json!(id));
    at_position(&mut item, args);
    if let Some(size) = args.flag("size") {
        let (width, height) = pair(size);
        if let Some(width) = width {
            item.insert("width".to_owned(), num(width));
        }
        if let Some(height) = height {
            item.insert("height".to_owned(), num(height));
        }
    }
    if let Some(preset) = args.flag("preset") {
        item.insert(
            "presetIndex".to_owned(),
            number(preset).map_or(Value::Null, num),
        );
    }
    for (flag, key) in [("url", "url"), ("text", "text"), ("color", "color")] {
        if let Some(value) = args.flag(flag).filter(|value| !value.is_empty()) {
            item.insert(key.to_owned(), json!(value));
        }
    }
    for (switch, key, value) in [
        ("landscape", "orientation", json!("landscape")),
        ("portrait", "orientation", json!("portrait")),
        ("device-frame", "showDeviceFrame", json!(true)),
        ("no-device-frame", "showDeviceFrame", json!(false)),
    ] {
        if args.switch(switch) {
            item.insert(key.to_owned(), value);
        }
    }
    if let Some(gap) = args.flag("gap") {
        item.insert("layoutGap".to_owned(), number(gap).map_or(Value::Null, num));
    }
    let Applied {
        created, updated, ..
    } = patch::apply(client, &entities(&[Value::Object(item)]), &Batch::default())?;
    output::json(&json!({ "created": created, "updated": updated }));
    Ok(0)
}

/// `delete <id...>`, or `delete --json` with `[{id}, ...]` on stdin.
pub(crate) fn delete(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let ids: Vec<Value> = if args.switch("json") {
        let items: Vec<Value> =
            serde_json::from_str(&read_stdin()?).context("delete: items are not JSON")?;
        items.iter().map(|item| item["id"].clone()).collect()
    } else if !args.positional.is_empty() {
        args.positional.iter().map(|id| json!(id)).collect()
    } else {
        return usage("specular delete <id> [id...] or specular delete --json");
    };
    let applied = patch::apply(client, &json!({ "delete": ids }), &Batch::default())?;
    output::json(&json!({ "deleted": applied.deleted }));
    Ok(0)
}

/// `link <from> <to> [--label]`, or a JSON edges array on stdin.
pub(crate) fn link(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let edges = match args.positional.as_slice() {
        [from, to, ..] => {
            let mut edge = json!({ "fromEntityId": from, "toEntityId": to, "kind": "connection" });
            if let Some(label) = args.flag("label") {
                edge["label"] = json!(label);
            }
            json!([edge])
        }
        [] if !stdin_is_terminal() => {
            serde_json::from_str(&read_stdin()?).context("link: edges are not JSON")?
        }
        _ => {
            return usage(
                "specular link <fromId> <toId> [--label <text>]  (or pipe a JSON edges array \
                 on stdin)",
            );
        }
    };
    let applied = patch::apply(client, &json!({ "edges": edges }), &Batch::default())?;
    output::json(&json!({ "edgeIds": applied.edges }));
    Ok(0)
}

/// `group <id...> [--label]`.
pub(crate) fn group(client: &Client, args: &Args) -> anyhow::Result<i32> {
    if args.positional.is_empty() {
        return usage("specular group <entityId> [entityId...]");
    }
    let mut item = json!({ "kind": "group", "entityIds": args.positional });
    if let Some(label) = args.flag("label") {
        item["label"] = json!(label);
    }
    let applied = patch::apply(client, &entities(&[item]), &Batch::default())?;
    output::json(&json!({ "id": applied.created.first(), "entityIds": args.positional }));
    Ok(0)
}
