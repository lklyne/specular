//! The headless mutation surface: what the `specular` CLI and agents drive
//! the app through.
//!
//! ```text
//! Request + &App -> Plan::Answer(Response)             a read
//!                -> Plan::Run { event, pending }       a write
//! ```
//!
//! [`Api::plan`] is pure. A read is answered from the [`App`] as it stands.
//! A write becomes an [`Event::Api`] for
//! [`update`](specular_interact::update), the same door the window's input
//! goes through, so it runs the same [`Command`](specular_doc::Command)s as
//! the UI and is one undo step. The event carries a ticket; when `update`
//! returns the [`Effect::ApiReply`](specular_interact::Effect::ApiReply)
//! with that ticket, [`Pending::finish`] reads the answer off the `App`.
//!
//! The routes, the secret header, the port and the discovery file are the
//! Electron app's (`src/main/routes/`), so its CLI talks to this app
//! unchanged. Nothing here opens a socket: the shell owns the server.

mod act;
mod annotations;
mod canvas;
pub mod cdp;
mod facts;
mod host;
mod http;
mod ids;
mod pages;
mod patch;
mod placement;
mod presets;
mod reply;
mod tabs;
mod tasks;
mod unported;

use std::path::PathBuf;

use serde_json::{Value, json};
use specular_doc::{EntityId, Rect};
use specular_interact::{ApiCall, ApiRun, App, CanvasId, Event};

pub use self::facts::Facts;
pub use self::host::Host;
pub use self::http::{Method, Request, Response, percent_decode};
use self::ids::Ids;
pub use self::pages::CdpAsk;
pub use self::reply::Pending;
use self::reply::Reply;

/// The API version the CLI checks the discovery file and `/health` for.
pub const VERSION: &str = "1";
/// The port the Electron app listens on, and this app when it is free.
pub const DEFAULT_PORT: u16 = 29979;
/// The header that carries the secret from the discovery file.
pub const SECRET_HEADER: &str = "x-specular-secret";
/// The header that carries a `--tab` ref, percent-encoded.
pub const TAB_HEADER: &str = "x-specular-tab";
/// The discovery file's name, under `~/.specular/`.
pub const DISCOVERY_FILE: &str = "specular-mcp.json";
/// The discovery file this app writes when the Electron app holds the
/// port, so the two never overwrite each other's.
pub const NATIVE_DISCOVERY_FILE: &str = "specular-native-mcp.json";

/// What the discovery file holds: where the server is and the secret every
/// request must carry.
pub fn discovery(port: u16, secret: &str) -> Value {
    json!({ "port": port, "secret": secret, "version": VERSION })
}

/// What to do about a request.
#[derive(Debug, Clone, PartialEq)]
pub enum Plan {
    /// Send this answer. Nothing changes.
    Answer(Response),
    /// Send `event` through `update`, then give the outcome that comes
    /// back to [`Pending::finish`].
    Run {
        /// The event, an [`Event::Api`].
        event: Event,
        /// The answer in waiting.
        pending: Pending,
    },
    /// Draw part of the canvas and answer with the PNG. Only the shell
    /// can: it holds the renderer.
    Screenshot(Screenshot),
    /// Answer with `body`, with the picture `shot` asks for under
    /// `metadata.regionScreenshot` when it can be drawn. A comment is worth
    /// reading without its picture, so a failed draw is not an error.
    Annotated {
        /// The annotation.
        body: Value,
        /// The picture of its region.
        shot: Screenshot,
    },
    /// Ask the shell about a page's devtools endpoint, which it serves.
    Cdp(CdpAsk),
}

/// A request for a picture of the canvas.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Screenshot {
    /// Where to write the PNG. Without one the answer carries it as base64.
    pub path: Option<PathBuf>,
    /// What to draw.
    pub area: ShotArea,
}

/// What a [`Screenshot`] shows.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ShotArea {
    /// The canvas as the window shows it.
    #[default]
    Window,
    /// This rect of the canvas, with everything on it.
    Canvas(Rect),
    /// One page at its own size.
    Page {
        /// The page entity.
        page: EntityId,
        /// With its border and title and whatever lies over it, `padding`
        /// canvas units around it. Without, the page's pixels alone.
        chrome: bool,
        /// The margin drawn around the page when `chrome` is set.
        padding: f64,
    },
}

/// What a route handler decided.
pub(crate) enum Step {
    Answer(Value),
    Run(ApiRun, Reply),
    Screenshot(Screenshot),
    Annotated(Value, Screenshot),
    Cdp(CdpAsk),
}

/// The API's state between requests: the sequences ids and tickets are
/// drawn from.
#[derive(Debug, Clone)]
pub struct Api {
    ids: Ids,
    ticket: u64,
}

impl Api {
    /// An API whose new ids are drawn from a sequence `id_seed` starts;
    /// pass something that differs between launches.
    pub fn new(id_seed: u64) -> Self {
        Self {
            ids: Ids::new(id_seed),
            ticket: 0,
        }
    }

    /// Decides what `request` does to `app`, with no facts about the disk.
    /// Changes neither.
    pub fn plan(&mut self, app: &App, request: &Request) -> Plan {
        self.plan_with(app, request, &Facts::default())
    }

    /// [`plan`](Self::plan) with what the host found on disk.
    pub fn plan_with(&mut self, app: &App, request: &Request, facts: &Facts) -> Plan {
        let segments: Vec<&str> = request.path.split('/').filter(|s| !s.is_empty()).collect();
        let canvas = match background_target(app, request, &segments) {
            Ok(canvas) => canvas,
            Err(response) => return Plan::Answer(response),
        };
        // A background canvas is read as if it were the active one.
        let scoped = canvas.as_ref().and_then(|id| app.background(id));
        match self.route(
            scoped.as_ref().unwrap_or(app),
            app,
            request,
            &segments,
            facts,
        ) {
            Ok(Step::Answer(body)) => Plan::Answer(Response::ok(body)),
            Ok(Step::Run(run, reply)) => {
                self.ticket += 1;
                let ticket = self.ticket;
                let call = ApiCall {
                    ticket,
                    canvas: canvas.clone(),
                    run,
                };
                Plan::Run {
                    event: Event::Api(call),
                    pending: Pending::new(ticket, reply, canvas),
                }
            }
            Ok(Step::Screenshot(shot)) => Plan::Screenshot(shot),
            Ok(Step::Annotated(body, shot)) => Plan::Annotated { body, shot },
            Ok(Step::Cdp(ask)) => Plan::Cdp(ask),
            Err(response) => Plan::Answer(response),
        }
    }

    /// Routes `request`. `app` is the canvas it is about: the user's app,
    /// or one standing in for the background canvas `--tab` named. `user`
    /// is always the user's app, which knows every canvas.
    fn route(
        &mut self,
        app: &App,
        user: &App,
        request: &Request,
        segments: &[&str],
        facts: &Facts,
    ) -> Result<Step, Response> {
        use Method::{Delete, Get, Post};
        let ids = &mut self.ids;
        match (request.method, segments) {
            (Get, ["health"]) => Ok(Step::Answer(json!({ "version": VERSION }))),
            (Get, ["canvas"]) => canvas::read(app, user),
            (Get, ["tabs"]) => Ok(Step::Answer(tabs::identity(user))),
            (Post, ["tabs"]) => Ok(tabs::create(request)),
            (Post, ["tabs", "delete"]) => tabs::delete(user, request),
            (Post, ["tabs", "switch"]) => tabs::switch(user, request),
            (Post, ["canvas", "apply"]) => patch::apply(ids, app, &request.body, facts),
            (Post, ["edges", "create"]) => patch::create_edges(ids, app, &request.body),
            (Post, ["edges", "delete"]) => patch::delete_edges(ids, app, request),
            (Post, ["groups", "create"]) => patch::create_group(ids, app, &request.body),
            (Post, ["groups", "ungroup"]) => act::ungroup(app, request),
            (Get, ["selection"]) => Ok(Step::Answer(act::selection(app))),
            (Post, ["selection", "annotate"]) => annotations::annotate_selection(ids, app, request),
            (Post, ["selection", verb]) => act::select(app, request, verb),
            (Post, ["camera", "focus"]) => Ok(act::focus(app, request)),
            (Post, ["stack-order", verb]) => act::stack_order(app, request, verb),
            (Post, ["history", verb]) => act::history(app, request, verb),
            (Post, ["tasks", "apply"]) => tasks::apply(ids, app, &request.body),
            (Post, ["layout", "find-placement"]) => Ok(placement::find(app, &request.body)),
            (Post, ["layout", "batch-placement"]) => placement::batch(app, &request.body),
            (Post, ["layout", "apply-directive"]) => placement::directive(app, &request.body),
            (Get, ["pages", id, "cdp-target"]) => pages::cdp_target(app, id),
            (Post, ["pages", id, "snapshot-seen"]) => pages::snapshot_seen(app, id),
            (Post, ["pages", "screenshot"]) => pages::screenshot(app, request, false),
            (Post, ["pages", "screenshot-composite"]) => pages::screenshot(app, request, true),
            (Post, ["pages", id, verb @ ("back" | "forward" | "reload")]) => {
                pages::navigate(app, id, verb)
            }
            (Get, ["annotations"]) => Ok(Step::Answer(annotations::list(app, request))),
            (Get, ["annotations", id]) => annotations::detail(app, id),
            (Post, ["annotations"]) => annotations::create(ids, app, request),
            (Post, ["annotations", id, verb]) => annotations::respond(app, request, id, verb),
            (Delete, ["annotations", id]) => annotations::delete(app, id),
            (Post, ["window", "screenshot"]) => Ok(Step::Screenshot(Screenshot {
                path: request.text("path").map(PathBuf::from),
                area: ShotArea::Window,
            })),
            _ => unported::answer(request, segments),
        }
    }
}

/// The background canvas `request` is for: the one its `--tab` ref names,
/// when that is not the canvas the user is looking at. `None` runs on the
/// active canvas. A ref on a route that is not about one canvas is an
/// error, since a write that lands on the wrong canvas with no signal is
/// the bug `--tab` exists to prevent.
fn background_target(
    app: &App,
    request: &Request,
    segments: &[&str],
) -> Result<Option<CanvasId>, Response> {
    let Some(tab_ref) = request.tab.as_deref().filter(|tab| !tab.is_empty()) else {
        return Ok(None);
    };
    let scoped = matches!(segments, ["canvas"] | ["canvas", "apply"] | ["layout", _]);
    if !scoped {
        return Err(Response::bad_request(format!(
            "--tab is not supported for {} {}",
            request.method.name(),
            request.path
        )));
    }
    let id = tabs::target(app, tab_ref)?;
    Ok(Some(id).filter(|id| *id != app.space().active().id))
}
