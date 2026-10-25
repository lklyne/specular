//! [`Host`]: whatever owns the [`App`] the API talks to. The shell is one
//! and a test's scripted app is another, so both answer a request the same
//! way.

use serde_json::Value;
use specular_interact::{ApiOutcome, App, DroppedFile, Event};

use crate::facts::Facts;
use crate::{Api, CdpAsk, PageScript, Plan, Request, Response, Screenshot};

/// The owner of an [`App`]: it can show it, run an event through `update`
/// on it, and draw it.
pub trait Host {
    /// The app as it stands.
    fn app(&self) -> &App;

    /// Sends `event`, an [`Event::Api`], through `update`, runs the effects
    /// that come back, and returns the outcome its
    /// [`Effect::ApiReply`](specular_interact::Effect::ApiReply) carried.
    fn run(&mut self, event: Event) -> Option<ApiOutcome>;

    /// Draws what `shot` asks for and returns the response body:
    /// `mimeType`, `width`, `height`, and `path` or `base64`. The error is
    /// why it could not.
    fn screenshot(&mut self, shot: &Screenshot) -> Result<Value, String>;

    /// Answers a question about a page's devtools endpoint. For
    /// [`CdpAsk::Target`] the body has `webSocketDebuggerUrl`, `generation`
    /// and `lastSnapshotGeneration`; for [`CdpAsk::SnapshotSeen`], `ok` and
    /// `generation`.
    fn cdp(&mut self, ask: &CdpAsk) -> Result<Value, Response>;

    /// Runs `script` in its page and calls `reply` with
    /// [`PageScript::answer`] once the page has answered, or with why it
    /// could not. A host with no pages to run it in says so at once.
    fn page_script(&mut self, script: PageScript, reply: LaterReply) {
        let _ = script;
        reply(Response::error(
            501,
            "this host cannot run a script in a page",
        ));
    }

    /// What a `file` create needs to know about `path`, which may be
    /// relative to the space folder: the same facts a dropped file has.
    /// `None` when there is no such file. A host with no disk finds none.
    fn inspect_file(&self, _path: &str) -> Option<DroppedFile> {
        None
    }

    /// The names of what is directly in the space folder, for naming a new
    /// Document so that it takes no file's place.
    fn space_entries(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Where an answer that comes later goes.
pub type LaterReply = Box<dyn FnOnce(Response) + Send>;

/// What [`Api::respond`] decided.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    /// This response, now.
    Now(Response),
    /// A script to run in a page; the response comes when the page answers.
    Later(PageScript),
}

impl Api {
    /// Answers `request` against `host`, calling `reply` with the response:
    /// at once, or once a page has answered a script.
    pub fn answer_to(&mut self, host: &mut impl Host, request: &Request, reply: LaterReply) {
        match self.respond(host, request) {
            Answer::Now(response) => reply(response),
            Answer::Later(script) => host.page_script(script, reply),
        }
    }

    /// Answers `request` against `host` now. A request that needs a page's
    /// answer is refused; [`answer_to`](Self::answer_to) waits for it.
    pub fn answer(&mut self, host: &mut impl Host, request: &Request) -> Response {
        match self.respond(host, request) {
            Answer::Now(response) => response,
            Answer::Later(_) => Response::error(501, "this request waits for a page's answer"),
        }
    }

    /// A read from `host`'s app or a write by running one event on it, or
    /// the script a page has to run first.
    pub fn respond(&mut self, host: &mut impl Host, request: &Request) -> Answer {
        let facts = Facts::gather(&*host, request);
        Answer::Now(match self.plan_with(host.app(), request, &facts) {
            Plan::PageScript(script) => return Answer::Later(script),
            Plan::Answer(response) => response,
            Plan::Run { event, pending } => match host.run(event) {
                Some(outcome) => pending.finish(host.app(), &outcome),
                None => Response::error(500, "the app did not answer the request"),
            },
            Plan::Screenshot(shot) => match host.screenshot(&shot) {
                Ok(body) => Response::ok(body),
                Err(reason) => Response::error(500, reason),
            },
            Plan::Annotated { mut body, shot } => {
                let picture = host.screenshot(&shot).ok();
                if let Some(base64) = picture.as_ref().and_then(|shot| shot["base64"].as_str()) {
                    if !body["metadata"].is_object() {
                        body["metadata"] = serde_json::json!({});
                    }
                    body["metadata"]["regionScreenshot"] = serde_json::json!(base64);
                }
                Response::ok(body)
            }
            Plan::Cdp(ask) => match host.cdp(&ask) {
                Ok(mut body) => {
                    ask.describe(&mut body);
                    Response::ok(body)
                }
                Err(response) => response,
            },
        })
    }
}
