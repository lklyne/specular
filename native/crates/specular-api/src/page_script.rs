//! `POST /pages/snapshot` and `/pages/agent-snapshot`: the page's DOM, read
//! by a script run in the page, as the Electron app's preload reads it.
//!
//! ```text
//! route -> Plan::PageScript --host--> Runtime.evaluate on the page's own devtools channel
//! answer <- PageScript::answer <-------- the page's reply, as a devtools message
//! ```
//!
//! The page answers on its own time, so this is the one kind of request the
//! host answers later: [`Host::page_script`](crate::Host::page_script) is
//! handed where the answer goes.

use serde_json::{Value, json};
use specular_doc::EntityId;
use specular_interact::App;

use crate::{Request, Response, Step};

/// The script, a function of `(maxDepth, structured)`.
const SNAPSHOT_JS: &str = include_str!("page_script/snapshot.js");
/// How deep the walk goes unless the caller says: the Electron default.
const DEFAULT_DEPTH: u64 = 10;

/// Which reading of the DOM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotKind {
    /// An indented outline, one element a line.
    Outline,
    /// A flat list of nodes with `@eN` refs, bounds and paths, for an agent.
    Agent,
}

/// A script to run in a page, and how its result is answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageScript {
    /// The page entity.
    pub page: EntityId,
    /// Which reading.
    pub kind: SnapshotKind,
    /// How many levels below `<body>` are read.
    pub max_depth: u64,
}

impl PageScript {
    /// The devtools message that runs the script, with the id `id`.
    pub fn devtools_message(&self, id: i32) -> String {
        let structured = self.kind == SnapshotKind::Agent;
        let expression = format!("({SNAPSHOT_JS})({}, {structured})", self.max_depth);
        json!({
            "id": id,
            "method": "Runtime.evaluate",
            "params": { "expression": expression, "returnByValue": true },
        })
        .to_string()
    }

    /// The response to the page's answer, `message`: the devtools reply to
    /// [`devtools_message`](Self::devtools_message).
    pub fn answer(&self, message: &Value) -> Response {
        if let Some(error) = message.get("error") {
            let reason = error["message"]
                .as_str()
                .unwrap_or("the page refused the script");
            return Response::error(500, format!("snapshot failed: {reason}"));
        }
        let result = &message["result"];
        if let Some(thrown) = result.get("exceptionDetails") {
            let reason = (thrown["exception"]["description"].as_str())
                .or(thrown["text"].as_str())
                .unwrap_or("the script threw");
            return Response::error(500, format!("snapshot failed: {reason}"));
        }
        let value = result["result"]["value"].clone();
        let snapshot = match self.kind {
            SnapshotKind::Outline => value,
            SnapshotKind::Agent => agent_snapshot(&self.page, &value),
        };
        Response::ok(json!({ "snapshot": snapshot }))
    }
}

/// The structured snapshot as the Electron app's `normalizeAgentSnapshot`
/// leaves it: the page's id added, and every node missing a required field
/// dropped.
fn agent_snapshot(page: &EntityId, raw: &Value) -> Value {
    let nodes: Vec<Value> = (raw["nodes"].as_array().into_iter().flatten())
        .filter(|node| {
            let bounds = &node["bounds"];
            node["ref"].is_string()
                && node["depth"].is_number()
                && node["tagName"].is_string()
                && ["x", "y", "width", "height"]
                    .iter()
                    .all(|side| bounds[side].is_number())
                && node["interactive"].is_boolean()
                && node["elementPath"].is_string()
                && node["fullPath"].is_string()
        })
        .map(|node| {
            let mut node = node.clone();
            if !node["parentRef"].is_string() {
                node["parentRef"] = Value::Null;
            }
            node
        })
        .collect();
    json!({
        "pageId": page,
        "url": raw["url"].as_str().unwrap_or("about:blank"),
        "title": raw["title"].as_str().unwrap_or(""),
        "nodes": nodes,
    })
}

/// `POST /pages/snapshot` and `/pages/agent-snapshot`. The outline reads
/// the selected page when `pageId` names none; the agent's needs one.
pub(crate) fn snapshot(app: &App, request: &Request, kind: SnapshotKind) -> Result<Step, Response> {
    let page = match (request.text("pageId"), kind) {
        (Some(id), _) => crate::pages::find(app, id)?,
        (None, SnapshotKind::Agent) => return Err(Response::bad_request("pageId is required")),
        (None, SnapshotKind::Outline) => crate::pages::selected(app)?,
    };
    let max_depth = request.body["maxDepth"].as_u64().unwrap_or(DEFAULT_DEPTH);
    Ok(Step::PageScript(PageScript {
        page,
        kind,
        max_depth,
    }))
}
