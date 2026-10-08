//! The page routes: history verbs, the page's devtools endpoint, and
//! pictures of one page.
//!
//! The Electron app has no route for `back`, `forward` and `reload`: the CLI
//! sends them over the page's CDP connection. They are routes here too, so
//! a caller with no CDP client can still move a page.

use serde_json::{Value, json};
use specular_core::PageNav;
use specular_doc::{EntityId, Kind};
use specular_interact::{ApiRun, App};

use crate::cdp::Target;
use crate::reply::Reply;
use crate::{Request, Response, Screenshot, ShotArea, Step};

/// The margin `screenshot-composite` draws around a page unless told.
const COMPOSITE_PADDING: f64 = 24.0;

/// A question about a page's devtools endpoint, for the shell that serves
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CdpAsk {
    /// Where the page's own CDP websocket is.
    Target {
        /// The page entity.
        page: EntityId,
        /// The page as a CDP client is told of it.
        target: Target,
    },
    /// An agent has just read the page: remember which load it saw, so a
    /// later click on a ref from before a navigation can be warned about.
    SnapshotSeen {
        /// The page entity.
        page: EntityId,
    },
}

impl CdpAsk {
    /// Adds what the route knew before the shell was asked to the shell's
    /// answer: the fields the Electron route sends beside the websocket.
    pub(crate) fn describe(&self, body: &mut Value) {
        if let Self::Target { page, target } = self {
            body["pageId"] = json!(page);
            body["targetId"] = json!(target.id);
            body["url"] = json!(target.url);
            body["title"] = json!(target.title);
        }
    }
}

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

/// `GET /pages/<id>/cdp-target`: a websocket that speaks CDP for this page
/// and no other. The CLI hands it to agent-browser, which treats it as a
/// browser with one page in it.
pub(crate) fn cdp_target(app: &App, id: &str) -> Result<Step, Response> {
    let page = find(app, id)?;
    let state = app.page_state(&page);
    let stored = app
        .document()
        .entity(&page)
        .and_then(|entity| match &entity.kind {
            Kind::Page(page) => Some(page.url.clone()),
            Kind::Text(_) | Kind::Shape(_) | Kind::File(_) | Kind::Group(_) | Kind::Drawing(_) => {
                None
            }
        });
    let target = Target {
        id: id.to_owned(),
        url: (state.and_then(|state| state.url.clone()))
            .or(stored)
            .unwrap_or_default(),
        title: state.map(|state| state.title.clone()).unwrap_or_default(),
    };
    Ok(Step::Cdp(CdpAsk::Target { page, target }))
}

/// `POST /pages/<id>/snapshot-seen`.
pub(crate) fn snapshot_seen(app: &App, id: &str) -> Result<Step, Response> {
    Ok(Step::Cdp(CdpAsk::SnapshotSeen {
        page: find(app, id)?,
    }))
}

/// `POST /pages/screenshot` and `/pages/screenshot-composite`: a picture of
/// the page `pageId` names, or of the selected page. The first is the
/// page's pixels alone, as the Electron app's `capturePage` gives them; the
/// second is the page as the canvas shows it, border and all.
pub(crate) fn screenshot(app: &App, request: &Request, chrome: bool) -> Result<Step, Response> {
    let page = if let Some(id) = request.text("pageId") {
        find(app, id)?
    } else {
        let mut selected = app.session().selection.entities();
        (selected.next().filter(|_| selected.next().is_none()))
            .filter(|id| app.page_placement(id).is_some())
            .cloned()
            .ok_or_else(|| Response::bad_request("pageId is required"))?
    };
    let padding = if chrome {
        (request.body["padding"].as_f64()).map_or(COMPOSITE_PADDING, |padding| padding.max(0.0))
    } else {
        0.0
    };
    Ok(Step::Screenshot(Screenshot {
        path: None,
        area: ShotArea::Page {
            page,
            chrome,
            padding,
        },
    }))
}
