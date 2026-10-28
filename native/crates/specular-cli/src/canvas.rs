//! The verbs that read the canvas or act on what is on it without a patch:
//! `canvas`, `tab`, `selection`, `find-placement`, `breakpoints`,
//! `arrange`, `focus`, `unlink`, `ungroup` and `auto-layout`.

use serde_json::{Value, json};

use crate::args::Args;
use crate::client::Client;
use crate::js::{num, number, number_or};
use crate::{output, usage};

const ANCHOR: &str = "selection_or_empty_region";

/// `canvas`: the canvas as a JSON Canvas document.
pub(crate) fn canvas(client: &Client, _args: &Args) -> anyhow::Result<i32> {
    output::json(&client.get("/canvas")?);
    Ok(0)
}

/// `tab`, `tab new`, `tab switch`, `tab delete`. Only `switch` moves what
/// the user is looking at.
pub(crate) fn tab(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let rest = args.words(1);
    let answer = match args.nth(0) {
        None => client.get("/tabs")?,
        Some("new") if rest.is_empty() => return usage("specular tab new <name>"),
        Some("new") => client.post("/tabs", &json!({ "name": rest }))?,
        Some(sub @ ("switch" | "delete")) if rest.is_empty() => {
            return usage(&format!("specular tab {sub} <tab-id|tab-name>"));
        }
        Some(sub @ ("switch" | "delete")) => {
            client.post(&format!("/tabs/{sub}"), &json!({ "ref": rest }))?
        }
        Some(_) => {
            return usage(
                "specular tab [new <name> | switch <tab-id|tab-name> | delete <tab-id|tab-name>]",
            );
        }
    };
    output::json(&answer);
    Ok(0)
}

/// `selection`.
pub(crate) fn selection(client: &Client, _args: &Args) -> anyhow::Result<i32> {
    output::json(&client.get("/selection")?);
    Ok(0)
}

/// `find-placement`: a free spot for a box.
pub(crate) fn find_placement(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let body = json!({
        "width": num(number_or(args.flag("width"), 800.0)),
        "height": num(number_or(args.flag("height"), 600.0)),
        "anchor": args.flag("anchor").unwrap_or(ANCHOR),
    });
    output::json(&client.post("/layout/find-placement", &body)?);
    Ok(0)
}

/// `breakpoints <url>`: one page per preset in a group. The app picks the
/// presets when none are named.
pub(crate) fn breakpoints(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let Some(url) = args.nth(0) else {
        return usage("specular breakpoints <url>");
    };
    let mut input = json!({ "url": url });
    if let Some(presets) = args.flag("presets") {
        input["presets"] = json!(presets.split(',').collect::<Vec<_>>());
    }
    if let Some(label) = args.flag("label") {
        input["label"] = json!(label);
    }
    let body = json!({
        "taskKind": "breakpoint_map",
        "input": input,
        "options": {
            "anchor": args.flag("anchor").unwrap_or(ANCHOR),
            "focus": !args.switch("no-focus"),
        },
    });
    output::json(&client.post("/tasks/apply", &body)?);
    Ok(0)
}

/// `arrange <row|column|grid|flow> [id...]`: tidy in place, or pack at
/// `--gap`; `flow` lays them out along their edges, running `--dir`. No ids
/// arranges the selection.
pub(crate) fn arrange(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let Some(mode @ ("row" | "column" | "grid" | "flow")) = args.nth(0) else {
        return usage(
            "specular arrange <row|column|grid|flow> [id...] [--gap m] [--cols N] [--dir LR|TB]  \
             (no ids uses current selection)",
        );
    };
    let mut body = json!({ "mode": mode });
    if let Some(dir) = args.flag("dir") {
        body["dir"] = json!(dir);
    }
    let ids = args.positional.get(1..).unwrap_or_default();
    if !ids.is_empty() {
        body["entityIds"] = json!(ids);
    }
    if let Some(gap) = args.flag("gap") {
        // A number is pixels; anything else is a spacing token.
        body["gap"] = number(gap).map_or_else(|| json!(gap), num);
    }
    if let Some(cols) = args.flag("cols") {
        body["cols"] = number(cols).map_or(Value::Null, num);
    }
    output::json(&client.post("/selection/arrange", &body)?);
    Ok(0)
}

/// `focus <id...>`: fit the view to them.
pub(crate) fn focus(client: &Client, args: &Args) -> anyhow::Result<i32> {
    if args.positional.is_empty() {
        return usage("specular focus <pageId> [pageId...]");
    }
    let body = json!({ "pageIds": args.positional });
    output::json(&client.post("/camera/focus", &body)?);
    Ok(0)
}

/// `unlink <edgeId...>`.
pub(crate) fn unlink(client: &Client, args: &Args) -> anyhow::Result<i32> {
    if args.positional.is_empty() {
        return usage("specular unlink <edgeId> [edgeId...]");
    }
    let body = json!({ "edgeIds": args.positional });
    output::json(&client.post("/edges/delete", &body)?);
    Ok(0)
}

/// `ungroup <groupId>`.
pub(crate) fn ungroup(client: &Client, args: &Args) -> anyhow::Result<i32> {
    let Some(group) = args.nth(0) else {
        return usage("specular ungroup <groupId>");
    };
    output::json(&client.post("/groups/ungroup", &json!({ "groupId": group }))?);
    Ok(0)
}

/// `auto-layout <id...>`: a managed row or column of these, or one group
/// made managed in place.
pub(crate) fn auto_layout(client: &Client, args: &Args) -> anyhow::Result<i32> {
    if args.positional.is_empty() {
        return usage(
            "specular auto-layout <entityId> [entityId...] [--gap N]  (or a single groupId)",
        );
    }
    let mut body = match args.positional.as_slice() {
        [group] if group.starts_with("group_") => json!({ "groupId": group }),
        ids => json!({ "entityIds": ids }),
    };
    if let Some(label) = args.flag("label") {
        body["label"] = json!(label);
    }
    if let Some(gap) = args.flag("gap") {
        body["gap"] = number(gap).map_or(Value::Null, num);
    }
    output::json(&client.post("/groups/auto-layout", &body)?);
    Ok(0)
}
