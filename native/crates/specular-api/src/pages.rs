//! The page routes: history verbs and the page's debugging address.
//!
//! The Electron app has no route for `back`, `forward` and `reload`: the CLI
//! sends them over the page's CDP connection. A Rust page backend reports
//! its websocket instead of proxying it, so the verbs are routes here too.

use serde_json::json;
use specular_core::PageNav;
use specular_doc::{EntityId, Kind};
use specular_interact::{ApiRun, App};

use crate::reply::Reply;
use crate::{Response, Step};

/// The page `id`, or the 404 the Electron routes answer with.
fn find(app: &App, id: &str) -> Result<EntityId, Response> {
    let page = EntityId::from(id);
    if app.page_placement(&page).is_none() {
        return Err(Response::not_found(format!("Page not found: {id}")));
    }
    Ok(page)
}

/// `POST /pages/<id>/back`, `/forward` or `/reload`.
pub(crate) fn navigate(app: &App, id: &str, verb: &str) -> Result<Step, Response> {
    let nav = match verb {
        "back" => PageNav::Back,
        "forward" => PageNav::Forward,
        _ => PageNav::Reload,
    };
    let page = find(app, id)?;
    let reply = Reply::Fixed(json!({ "ok": true, "pageId": id }));
    Ok(Step::Run(ApiRun::Navigate { page, nav }, reply))
}

/// `GET /pages/<id>/cdp-target`: where the page's remote-debugging websocket
/// is. The body carries `webSocketDebuggerUrl` and `url`, the fields the
/// CLI's browse handler reads. It leaves out the navigation generations,
/// which this app does not count, so the CLI skips its stale-ref warning.
///
/// Only while the canvas has one page. The CLI hands the socket to
/// agent-browser, which connects to the port and drives the first page it
/// finds there, whichever page was named. The Electron app answers with a
/// proxy that shows that client one page; this app has none, so with several
/// pages the answer would send a click to the wrong one.
pub(crate) fn cdp_target(app: &App, id: &str) -> Result<Step, Response> {
    let page = find(app, id)?;
    let state = app.page_state(&page);
    if app.pages().nth(1).is_some() {
        let socket = state.and_then(|state| state.devtools_url.as_deref());
        return Err(Response::error(
            501,
            format!(
                "Not implemented in the Rust app with more than one page on the canvas: the \
                 browse verbs would drive the first page on the debugging port, not {id}. A \
                 CDP client can attach to the page itself at {}",
                socket.unwrap_or("its websocket, once the page reports one"),
            ),
        ));
    }
    let Some(socket) = state.and_then(|state| state.devtools_url.as_deref()) else {
        return Err(Response::error(
            503,
            format!("Page {id} has no debugging target yet: its browser has not reported one"),
        ));
    };
    let url = state.and_then(|state| state.url.clone()).or_else(|| {
        let entity = app.document().entity(&page)?;
        if let Kind::Page(page) = &entity.kind {
            Some(page.url.clone())
        } else {
            None
        }
    });
    let mut body = json!({ "webSocketDebuggerUrl": socket });
    if let Some(url) = url {
        body["url"] = json!(url);
    }
    Ok(Step::Answer(body))
}
