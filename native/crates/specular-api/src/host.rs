//! [`Host`]: whatever owns the [`App`] the API talks to. The shell is one
//! and a test's scripted app is another, so both answer a request the same
//! way.

use serde_json::Value;
use specular_interact::{ApiOutcome, App, Event};

use crate::{Api, Plan, Request, Response, Screenshot};

/// The owner of an [`App`]: it can show it, run an event through `update`
/// on it, and draw it.
pub trait Host {
    /// The app as it stands.
    fn app(&self) -> &App;

    /// Sends `event`, an [`Event::Api`], through `update`, runs the effects
    /// that come back, and returns the outcome its
    /// [`Effect::ApiReply`](specular_interact::Effect::ApiReply) carried.
    fn run(&mut self, event: Event) -> Option<ApiOutcome>;

    /// Draws the canvas and returns the response body: `mimeType`, `width`,
    /// `height`, and `path` or `base64`. The error is why it could not.
    fn screenshot(&mut self, shot: &Screenshot) -> Result<Value, String>;
}

impl Api {
    /// Answers `request` against `host`: a read from its app, a write by
    /// running one event on it.
    pub fn answer(&mut self, host: &mut impl Host, request: &Request) -> Response {
        match self.plan(host.app(), request) {
            Plan::Answer(response) => response,
            Plan::Run { event, pending } => match host.run(event) {
                Some(outcome) => pending.finish(host.app(), &outcome),
                None => Response::error(500, "the app did not answer the request"),
            },
            Plan::Screenshot(shot) => match host.screenshot(&shot) {
                Ok(body) => Response::ok(body),
                Err(reason) => Response::error(500, reason),
            },
        }
    }
}
