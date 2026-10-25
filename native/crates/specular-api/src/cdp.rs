//! One page's devtools endpoint, as a browser-level CDP client sees it.
//!
//! ```text
//! client  --Target.*-->  PageProxy              answered here: one target
//! client  --session-->   PageProxy  --> page    id rewritten, session dropped
//! client  <--session--   PageProxy  <-- page    id restored, session put back
//! ```
//!
//! A client such as agent-browser connects to what it takes for a browser:
//! it lists the targets, attaches to one and then talks to that session. A
//! [`PageProxy`] plays the browser for exactly one page. The `Target` domain
//! is answered here with that page alone, and everything addressed to the
//! session goes to the page's own devtools channel
//! ([`PageSource::devtools_send`](specular_core::PageSource::devtools_send)),
//! which no other page shares. So a client cannot see or drive another page
//! however many there are.
//!
//! One method is this app's own. A client that is about to act on the page
//! sends [`WILL_ACT`] with the point and waits, and the answer is held
//! ([`Out::Act`]) until the agent cursor is drawn there.
//!
//! Nothing here opens a socket or touches a page: messages come in as text
//! and go out as [`Out`].

mod browser;
mod target;

use self::browser::{refused_on_page, reply_error};
pub use self::target::Target;

use std::collections::HashMap;

use serde_json::{Map, Value, json};
use specular_core::DEVTOOLS_CLIENT_ID_BASE;

/// One connected client of a [`PageProxy`].
pub type ClientId = u64;

/// The JSON-RPC code for a request the server will not carry out.
const SERVER_ERROR: i32 = -32000;
/// The JSON-RPC code for parameters that name nothing.
const INVALID_PARAMS: i32 = -32602;

/// The first id of the app's own devtools messages to a page, such as a
/// snapshot's script. A client's ids are kept below it, so an answer is
/// never taken for the other's.
pub const APP_ID_BASE: i32 = i32::MAX - (1 << 24);

/// The method a client announces an act with before it dispatches input.
pub(crate) const WILL_ACT: &str = "Specular.willAct";

/// What a client is about to do to the page.
#[derive(Debug, Clone, PartialEq)]
pub struct WillAct {
    /// The verb, as the CLI spells it.
    pub action: String,
    /// Where, in the page's viewport CSS pixels.
    pub x: f64,
    /// See `x`.
    pub y: f64,
    /// The client's own name for its session.
    pub session: String,
    /// Whether the client holds the act until it is answered. A read does
    /// not: it is answered at once and the cursor follows.
    pub wait: bool,
    /// How far the page was scrolled when the point was measured, in CSS
    /// pixels, when the client says.
    pub scroll: Option<(f64, f64)>,
}

/// Where a message goes next.
#[derive(Debug, Clone, PartialEq)]
pub enum Out {
    /// To this client, as is.
    ToClient(ClientId, String),
    /// To the page's devtools channel. `id` is the message's own id there;
    /// if it cannot be sent, say so with [`PageProxy::refuse`].
    ToPage {
        /// The id the message carries to the page.
        id: i32,
        /// The message.
        message: String,
    },
    /// `client` announced an act. Send it `reply` once its cursor has
    /// arrived, or at once when the act does not wait.
    Act {
        /// The client waiting.
        client: ClientId,
        /// What it is about to do.
        act: WillAct,
        /// The answer that lets it.
        reply: String,
    },
}

/// A request the page has not answered yet.
#[derive(Debug)]
struct Waiting {
    client: ClientId,
    /// The id the client gave it.
    id: Value,
    /// The session the client sent it on, when that is one made here. A
    /// session the page made itself (a frame or a worker) is in the page's
    /// answer already.
    session: Option<String>,
}

#[derive(Debug, Default)]
struct Client {
    /// The sessions this client holds on the page, made here.
    sessions: Vec<String>,
}

/// The browser one page's clients talk to. See the module docs.
#[derive(Debug)]
pub struct PageProxy {
    target: Target,
    next_id: i32,
    next_session: u64,
    waiting: HashMap<i32, Waiting>,
    clients: HashMap<ClientId, Client>,
}

impl PageProxy {
    /// A proxy for the page `target` describes, with no clients.
    pub fn new(target: Target) -> Self {
        Self {
            target,
            next_id: DEVTOOLS_CLIENT_ID_BASE,
            next_session: 0,
            waiting: HashMap::new(),
            clients: HashMap::new(),
        }
    }

    /// The page as clients are told of it.
    pub const fn target(&self) -> &Target {
        &self.target
    }

    /// Follows the page to a new address or title.
    pub fn set_target(&mut self, url: &str, title: &str) {
        url.clone_into(&mut self.target.url);
        title.clone_into(&mut self.target.title);
    }

    /// Whether any client is connected.
    pub fn has_clients(&self) -> bool {
        !self.clients.is_empty()
    }

    /// A client connected.
    pub fn connect(&mut self, client: ClientId) {
        self.clients.entry(client).or_default();
    }

    /// A client went away. What it asked and was not answered is forgotten,
    /// so a late answer goes nowhere.
    pub fn disconnect(&mut self, client: ClientId) {
        self.clients.remove(&client);
        self.waiting.retain(|_, waiting| waiting.client != client);
    }

    /// Routes one message from `client`.
    pub fn from_client(&mut self, client: ClientId, text: &str) -> Vec<Out> {
        let Ok(Value::Object(mut message)) = serde_json::from_str::<Value>(text) else {
            return Vec::new();
        };
        let id = message.get("id").cloned().unwrap_or(Value::Null);
        let method = (message.get("method").and_then(Value::as_str))
            .unwrap_or_default()
            .to_owned();
        let session = (message.get("sessionId").and_then(Value::as_str)).map(str::to_owned);
        let held = (self.clients.get(&client))
            .is_some_and(|c| session.as_ref().is_some_and(|s| c.sessions.contains(s)));

        if method == WILL_ACT {
            let params = &message["params"];
            let act = WillAct {
                action: params["action"].as_str().unwrap_or_default().to_owned(),
                x: params["x"].as_f64().unwrap_or_default(),
                y: params["y"].as_f64().unwrap_or_default(),
                session: params["session"].as_str().unwrap_or_default().to_owned(),
                wait: params["wait"].as_bool().unwrap_or(true),
                scroll: (params["scrollX"].as_f64()).zip(params["scrollY"].as_f64()),
            };
            let mut reply = Map::new();
            reply.insert("id".to_owned(), id);
            reply.insert("result".to_owned(), json!({}));
            if let Some(session) = session {
                reply.insert("sessionId".to_owned(), json!(session));
            }
            let reply = Value::Object(reply).to_string();
            return vec![Out::Act { client, act, reply }];
        }
        if session.is_none() {
            let params = message.get("params").cloned().unwrap_or(Value::Null);
            if let Some(out) = self.browser_level(client, &id, &method, &params) {
                return out;
            }
        }
        if let Some(reason) = refused_on_page(&method) {
            let reply = reply_error(&id, session.as_deref(), SERVER_ERROR, reason);
            return vec![Out::ToClient(client, reply)];
        }
        // The page has one session, so the one made here is not named to it.
        if held {
            message.remove("sessionId");
        }
        let upstream = self.fresh_id();
        message.insert("id".to_owned(), json!(upstream));
        let session = session.filter(|_| held);
        self.waiting.insert(
            upstream,
            Waiting {
                client,
                id,
                session,
            },
        );
        vec![Out::ToPage {
            id: upstream,
            message: Value::Object(message).to_string(),
        }]
    }

    /// Routes one message from the page: an answer to the client that
    /// asked, an event to every session on the page.
    pub fn from_page(&mut self, text: &str) -> Vec<Out> {
        let Ok(Value::Object(mut message)) = serde_json::from_str::<Value>(text) else {
            return Vec::new();
        };
        if let Some(id) = message.get("id") {
            let Some(waiting) = (id.as_i64())
                .and_then(|id| i32::try_from(id).ok())
                .and_then(|id| self.waiting.remove(&id))
            else {
                return Vec::new();
            };
            message.insert("id".to_owned(), waiting.id);
            if let Some(session) = waiting.session {
                message.insert("sessionId".to_owned(), json!(session));
            }
            return vec![Out::ToClient(
                waiting.client,
                Value::Object(message).to_string(),
            )];
        }
        // An event of a session the page made itself already names it.
        if message.contains_key("sessionId") {
            let text = Value::Object(message).to_string();
            return (self.clients.iter())
                .filter(|(_, client)| !client.sessions.is_empty())
                .map(|(id, _)| Out::ToClient(*id, text.clone()))
                .collect();
        }
        let mut out = Vec::new();
        for (id, client) in &self.clients {
            for session in &client.sessions {
                message.insert("sessionId".to_owned(), json!(session));
                out.push(Out::ToClient(
                    *id,
                    Value::Object(message.clone()).to_string(),
                ));
            }
        }
        out
    }

    /// The error a client gets for the message `id` that could not be sent
    /// to the page.
    pub fn refuse(&mut self, id: i32, reason: &str) -> Option<Out> {
        let waiting = self.waiting.remove(&id)?;
        let reply = reply_error(
            &waiting.id,
            waiting.session.as_deref(),
            SERVER_ERROR,
            reason,
        );
        Some(Out::ToClient(waiting.client, reply))
    }

    fn fresh_id(&mut self) -> i32 {
        let id = self.next_id;
        self.next_id = (self.next_id.checked_add(1))
            .filter(|next| *next < APP_ID_BASE)
            .unwrap_or(DEVTOOLS_CLIENT_ID_BASE);
        id
    }
}
