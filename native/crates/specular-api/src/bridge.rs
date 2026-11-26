//! The page bridge's messages (ADR 0055, "The lightest extension is a
//! page"): `window.specular` in a page from the space folder, answered by
//! the API's own routes.
//!
//! ```text
//! page: specular.run(name, args) --binding--> {id, op, nonce, ..}
//!   Api::bridge -> Step::Run(event)  --update--> BridgeRun::finish -> answer script
//!               -> Step::Reply(answer script)            (state, commands, an error)
//! shell: the script, evaluated in the page --> the promise settles
//! ```
//!
//! `state`, `commands` and `run` are `GET /state`, `GET /commands` and
//! `POST /run` planned without a socket, so a page's edit is an
//! [`Event::Api`] through `update` and one undo step, refused the same way.
//! `subscribe` answers with the value now, and [`Subscriptions`] pushes it
//! again whenever its revision moves. Which pages get the bridge is
//! [`specular_interact::bridge_allowed`]; the transport is the shell's.

mod subscriptions;

pub use self::subscriptions::Subscriptions;

use serde::Deserialize;
use serde_json::{Value, json};
use specular_core::BRIDGE_BINDING;
use specular_interact::{ApiOutcome, App, Event, STATE_KEYS};

use crate::{Api, Method, Pending, Plan, Request, Response};

/// What makes `window.specular` in a page, over the binding.
const INSTALL_JS: &str = include_str!("bridge/install.js");

/// The ops a page can send.
const OPS: &str = "state, run, commands, subscribe, unsubscribe";

/// The script that puts `window.specular` in a document. Calls carry
/// `nonce`, which only this script's closure holds, so a frame the page
/// embeds cannot speak for it.
pub fn install_script(nonce: &str) -> String {
    INSTALL_JS
        .replace("__BINDING__", &json!(BRIDGE_BINDING).to_string())
        .replace("__NONCE__", &json!(nonce).to_string())
}

/// What a bridge call asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum BridgeStep {
    /// Run this script in the page now: the answer.
    Reply(String),
    /// Send `event` through `update`, then run in the page what
    /// [`BridgeRun::finish`] makes.
    Run {
        /// The event, an [`Event::Api`].
        event: Event,
        /// The answer in waiting.
        pending: BridgeRun,
    },
    /// Push `key` to the page whenever it moves, from now on, and run
    /// `reply` in it now.
    Subscribe {
        /// The key.
        key: &'static str,
        /// Its value now.
        value: Value,
        /// The answer, which carries the value.
        reply: String,
    },
    /// Stop pushing `key`, and run `reply` in the page.
    Unsubscribe {
        /// The key.
        key: String,
        /// The answer.
        reply: String,
    },
    /// Not a call this bridge's script made: it gets no answer.
    Ignore,
}

/// A page's `run`, waiting for its event to go through `update`.
#[derive(Debug, Clone, PartialEq)]
pub struct BridgeRun {
    id: u64,
    pending: Pending,
}

impl BridgeRun {
    /// The answer script, given how the event went; `None` when `update`
    /// did not say.
    pub fn finish(self, app: &App, outcome: Option<&ApiOutcome>) -> String {
        let response = match outcome {
            Some(outcome) => self.pending.finish(app, outcome),
            None => Response::error(500, "the app did not answer the request"),
        };
        answer(self.id, &response, |body| body)
    }
}

#[derive(Deserialize)]
struct Call {
    id: u64,
    nonce: String,
    op: String,
    #[serde(default)]
    key: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    args: Value,
}

impl Api {
    /// What the bridge call `payload` asks of `app`, from a page whose
    /// bridge was installed with `nonce`.
    pub fn bridge(&mut self, app: &App, payload: &str, nonce: &str) -> BridgeStep {
        let Ok(call) = serde_json::from_str::<Call>(payload) else {
            return BridgeStep::Ignore;
        };
        if call.nonce != nonce {
            return BridgeStep::Ignore;
        }
        let id = call.id;
        match call.op.as_str() {
            "state" => {
                let response = self.answer_now(app, &state_request(call.key.as_deref()));
                BridgeStep::Reply(answer(id, &response, |body| body))
            }
            "commands" => {
                let response = self.answer_now(app, &Request::get("/commands"));
                BridgeStep::Reply(answer(id, &response, |mut body| body["commands"].take()))
            }
            "run" => {
                let body = json!({ "name": call.name, "args": call.args });
                match self.plan(app, &Request::post("/run", body)) {
                    Plan::Run { event, pending } => BridgeStep::Run {
                        event,
                        pending: BridgeRun { id, pending },
                    },
                    Plan::Answer(response) => BridgeStep::Reply(answer(id, &response, |b| b)),
                    _ => BridgeStep::Reply(failed(id, "`run` planned no write")),
                }
            }
            "subscribe" => subscribe(id, call.key.as_deref(), app),
            "unsubscribe" => BridgeStep::Unsubscribe {
                key: call.key.unwrap_or_default(),
                reply: settled(id, Ok(Value::Null)),
            },
            other => BridgeStep::Reply(failed(
                id,
                &format!("no bridge op `{other}`; the ops: {OPS}"),
            )),
        }
    }

    /// The response to a read route, which plans no write.
    fn answer_now(&mut self, app: &App, request: &Request) -> Response {
        match self.plan(app, request) {
            Plan::Answer(response) => response,
            _ => Response::error(500, "a read planned a write"),
        }
    }
}

/// `GET /state`, for one key or every key.
fn state_request(key: Option<&str>) -> Request {
    let mut request = Request::new(Method::Get, "/state", json!({}));
    if let Some(key) = key {
        request.query.push(("key".to_owned(), key.to_owned()));
    }
    request
}

fn subscribe(id: u64, key: Option<&str>, app: &App) -> BridgeStep {
    let Some(key) = STATE_KEYS.iter().find(|known| Some(**known) == key) else {
        let named = key.unwrap_or("");
        let reason = format!(
            "no state key `{named}`; the keys: {}",
            STATE_KEYS.join(", ")
        );
        return BridgeStep::Reply(failed(id, &reason));
    };
    let value = specular_interact::state_key(app, key).unwrap_or(Value::Null);
    BridgeStep::Subscribe {
        key,
        reply: settled(id, Ok(value.clone())),
        value,
    }
}

/// The answer to call `id` from `response`: its body, through `shape`,
/// when it succeeded, and its `error` when it did not.
fn answer(id: u64, response: &Response, shape: impl FnOnce(Value) -> Value) -> String {
    if (200..300).contains(&response.status) {
        return settled(id, Ok(shape(response.body.clone())));
    }
    let reason = response.body["error"].as_str().unwrap_or("the app refused");
    failed(id, reason)
}

fn failed(id: u64, reason: &str) -> String {
    settled(id, Err(reason))
}

/// The script that settles the page's promise for call `id`.
fn settled(id: u64, result: Result<Value, &str>) -> String {
    let message = match result {
        Ok(value) => json!({ "id": id, "ok": true, "value": value }),
        Err(reason) => json!({ "id": id, "ok": false, "error": reason }),
    };
    receive(&message)
}

/// The script that gives the page `key`'s new value, for its subscribers.
pub fn push_script(key: &str, value: &Value) -> String {
    receive(&json!({ "key": key, "value": value }))
}

fn receive(message: &Value) -> String {
    format!("typeof window.__specularReceive === 'function' && window.__specularReceive({message})")
}
