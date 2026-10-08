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
mod host;
mod http;
mod ids;
mod patch;
mod placement;
mod presets;
mod reply;
mod unported;

use std::path::PathBuf;

use serde_json::{Value, json};
use specular_interact::{ApiCall, ApiRun, App, Event};

pub use self::host::Host;
pub use self::http::{Method, Request, Response, percent_decode};
use self::ids::Ids;
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

/// The one canvas the app has open, as the tab verbs name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    /// The tab's id.
    pub id: String,
    /// The tab's name: the canvas file's name without its extension.
    pub name: String,
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
    /// Draw the canvas as the window shows it and answer with the PNG.
    /// Only the shell can: it holds the renderer.
    Screenshot(Screenshot),
}

/// A request for a picture of the canvas.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Screenshot {
    /// Where to write the PNG. Without one the answer carries it as base64.
    pub path: Option<PathBuf>,
}

/// What a route handler decided.
pub(crate) enum Step {
    Answer(Value),
    Run(ApiRun, Reply),
    Screenshot(Screenshot),
}

/// The API's state between requests: the tab it answers for and the
/// sequences ids and tickets are drawn from.
#[derive(Debug, Clone)]
pub struct Api {
    tab: Tab,
    ids: Ids,
    ticket: u64,
}

impl Api {
    /// An API for the canvas `tab` names. `id_seed` starts the sequence new
    /// ids are drawn from; pass something that differs between launches.
    pub fn new(tab: Tab, id_seed: u64) -> Self {
        Self {
            tab,
            ids: Ids::new(id_seed),
            ticket: 0,
        }
    }

    /// Decides what `request` does to `app`. Changes neither.
    pub fn plan(&mut self, app: &App, request: &Request) -> Plan {
        match self.route(app, request) {
            Ok(Step::Answer(body)) => Plan::Answer(Response::ok(body)),
            Ok(Step::Run(run, reply)) => {
                self.ticket += 1;
                let ticket = self.ticket;
                Plan::Run {
                    event: Event::Api(ApiCall { ticket, run }),
                    pending: Pending::new(ticket, reply),
                }
            }
            Ok(Step::Screenshot(shot)) => Plan::Screenshot(shot),
            Err(response) => Plan::Answer(response),
        }
    }

    fn route(&mut self, app: &App, request: &Request) -> Result<Step, Response> {
        use Method::{Delete, Get, Post};
        let segments: Vec<&str> = request.path.split('/').filter(|s| !s.is_empty()).collect();
        self.check_tab(request, &segments)?;
        let ids = &mut self.ids;
        match (request.method, segments.as_slice()) {
            (Get, ["health"]) => Ok(Step::Answer(json!({ "version": VERSION }))),
            (Get, ["canvas"]) => canvas::read(app, &self.tab),
            (Get, ["tabs"]) => Ok(Step::Answer(canvas::tabs(app, &self.tab))),
            (Post, ["canvas", "apply"]) => patch::apply(ids, app, &request.body),
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
            (Post, ["layout", "find-placement"]) => Ok(placement::find(app, &request.body)),
            (Post, ["layout", "batch-placement"]) => placement::batch(app, &request.body),
            (Post, ["layout", "apply-directive"]) => placement::directive(app, &request.body),
            (Get, ["annotations"]) => Ok(Step::Answer(annotations::list(app, request))),
            (Get, ["annotations", id]) => annotations::detail(app, id),
            (Post, ["annotations"]) => annotations::create(ids, app, request),
            (Post, ["annotations", id, verb]) => annotations::respond(app, request, id, verb),
            (Delete, ["annotations", id]) => annotations::delete(app, id),
            (Post, ["window", "screenshot"]) => Ok(Step::Screenshot(Screenshot {
                path: request.text("path").map(PathBuf::from),
            })),
            _ => unported::answer(request, &segments),
        }
    }

    /// `--tab` names a canvas to write to without switching to it. This app
    /// has one canvas open, so the ref must name it, on a route that takes
    /// one.
    fn check_tab(&self, request: &Request, segments: &[&str]) -> Result<(), Response> {
        let Some(tab) = request.tab.as_deref().filter(|tab| !tab.is_empty()) else {
            return Ok(());
        };
        let scoped = matches!(segments, ["canvas"] | ["canvas", "apply"] | ["layout", _]);
        if !scoped {
            return Err(Response::bad_request(format!(
                "--tab is not supported for {} {}",
                request.method.name(),
                request.path
            )));
        }
        if tab == self.tab.id || tab == self.tab.name {
            return Ok(());
        }
        Err(Response::bad_request(format!(
            "no tab matches '{tab}'. Open tabs: {} ({})",
            self.tab.name, self.tab.id
        )))
    }
}
