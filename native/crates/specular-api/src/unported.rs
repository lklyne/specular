//! The routes this app answers without doing anything: the presence
//! bookkeeping the CLI sends with every verb, and every Electron route not
//! ported yet, which says so by name instead of `404`.

use serde_json::json;

use crate::{Method, Request, Response, Step};

/// The verbs behind an Electron route this app does not have, and why.
fn verbs(method: Method, segments: &[&str]) -> Option<(&'static str, &'static str)> {
    const LATER: &str = "not ported yet";
    Some(match (method, segments) {
        (_, ["pages", "create-at-position"]) => (
            "page duplication at a position",
            "use `specular add page <url> --at x,y`",
        ),
        (_, ["pages", _, "print-pdf"]) => (
            "`print-pdf`",
            "it needs an answer that waits for the page: `Page.printToPDF` over the page's \
             devtools channel, and the API answers within one turn of the event loop",
        ),
        (_, ["pages" | "debug", ..]) => (
            "the Electron app's own page reads (`/pages/snapshot`, `/pages/agent-snapshot`, \
             `/pages/query-elements`, `/pages/find-target`, `/debug/cdp-proxy`)",
            "use `specular snapshot -f <page>` and the other browse verbs, which go through \
             `/pages/<id>/cdp-target`",
        ),
        (_, ["selection", "arrange"]) => ("`arrange`", LATER),
        (_, ["selection", "enter-group" | "overlay-state"]) => ("entering a group", LATER),
        (_, ["groups", "auto-layout" | "reorder-child"]) => ("`auto-layout`", LATER),
        (_, ["groups", "delete"]) => ("group delete by route", "use `specular delete <id>`"),
        (_, ["tasks", "apply"]) => ("`breakpoints`", LATER),
        (_, ["tasks", "component-states"]) => ("`component-states`", LATER),
        (_, ["recording", ..]) => (
            "`record`",
            "it needs a frame tap on the compositor and a video encoder, neither of which exists",
        ),
        (_, ["design-system", ..]) => ("`design-system` and `register-design-system`", LATER),
        (_, ["annotations", "fix"]) => ("the comment fix loop", LATER),
        (_, ["perf", ..]) => ("perf tracing", "the Rust app has its own `--bench`"),
        (_, ["sidebar" | "text-entities" | "file-entities" | "drawing-entities"]) => {
            ("the per-kind read routes", "read `specular canvas` instead")
        }
        _ => return None,
    })
}

pub(crate) fn answer(request: &Request, segments: &[&str]) -> Result<Step, Response> {
    match (request.method, segments) {
        // The CLI reports a presence cursor and an MCP heartbeat. There is
        // no presence layer here, so they are accepted and dropped.
        (Method::Get, ["session", "presence"]) => Ok(Step::Answer(json!({ "cursors": [] }))),
        (
            Method::Post,
            ["session", "presence"]
            | ["session", "presence", "intent"]
            | ["mcp", "session", "open" | "ping" | "close"],
        ) => Ok(Step::Answer(json!({ "ok": true }))),
        _ => Err(match verbs(request.method, segments) {
            Some((what, why)) => Response::error(
                501,
                format!(
                    "not implemented in the native app ({why}): {what}. Route: {} {}",
                    request.method.name(),
                    request.path
                ),
            ),
            None => Response::not_found(format!(
                "Unknown route: {} {}",
                request.method.name(),
                request.path
            )),
        }),
    }
}
