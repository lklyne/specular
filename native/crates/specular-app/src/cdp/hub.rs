//! Every page's proxy and every client's socket, behind one lock.
//!
//! Three threads meet here. A connection thread brings a client's message
//! and leaves with what goes to the page queued for the event loop. The
//! event loop registers pages and drains that queue into the page backend.
//! The backend's devtools sink, on the main thread inside the backend's own
//! pump, brings a page's answer and writes it to the client's queue. None
//! of them calls another while holding the lock.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::{Value, json};
use specular_api::cdp::{APP_ID_BASE, Out, PageProxy, Target, WillAct};
use specular_api::{LaterReply, PageScript, Response};
use specular_core::PageId;
use specular_doc::EntityId;

use super::socket::{WsHandler, WsSender};

/// The path a page's websocket is served under, before its token.
pub(super) const PATH: &str = "/cdp/page/";

/// A client's message on its way to a page's devtools channel.
#[derive(Debug)]
pub(super) struct ToPage {
    pub(super) page: PageId,
    /// The message's id on that channel, to refuse it by.
    pub(super) id: i32,
    pub(super) message: String,
}

/// An act a client announced.
#[derive(Debug)]
pub(super) struct PendingAct {
    /// What [`Hub::go`] answers it by. `None` when it was answered already.
    pub(super) ticket: Option<u64>,
    pub(super) entity: EntityId,
    pub(super) act: WillAct,
}

/// What a client's message asks of the event loop.
#[derive(Debug)]
pub(super) enum ToLoop {
    Page(ToPage),
    Act(PendingAct),
}

/// One page with an endpoint.
struct Entry {
    entity: EntityId,
    /// The backend page hosting it now. A page hosted again gets a new one.
    page: PageId,
    proxy: PageProxy,
}

/// How many times a page has moved to another document, and which of those
/// an agent last read.
#[derive(Debug, Clone, Copy, Default)]
struct Generation {
    current: u64,
    snapshot: Option<u64>,
}

#[derive(Default)]
struct State {
    /// Each endpoint by the token in its address.
    pages: HashMap<String, Entry>,
    /// Each connection's token and socket.
    connections: HashMap<u64, (String, WsSender)>,
    next_connection: u64,
    generations: HashMap<EntityId, Generation>,
    /// The client and the answer each waiting act is owed, by ticket.
    acts: HashMap<u64, (u64, String)>,
    next_act: u64,
    /// Each script sent to a page by the app, by its message id.
    scripts: HashMap<i32, Waiting>,
    next_script: i32,
}

/// A script waiting for its page's answer.
struct Waiting {
    entity: EntityId,
    script: PageScript,
    reply: LaterReply,
}

impl State {
    fn deliver(&mut self, entry: (&EntityId, PageId), out: Vec<Out>, to_loop: &mut Vec<ToLoop>) {
        let (entity, page) = entry;
        for out in out {
            match out {
                Out::ToClient(client, text) => {
                    if let Some((_, socket)) = self.connections.get(&client) {
                        socket.send(text);
                    }
                }
                Out::ToPage { id, message } => {
                    to_loop.push(ToLoop::Page(ToPage { page, id, message }));
                }
                Out::Act { client, act, reply } => {
                    let ticket = if act.wait {
                        self.next_act += 1;
                        self.acts.insert(self.next_act, (client, reply));
                        Some(self.next_act)
                    } else {
                        if let Some((_, socket)) = self.connections.get(&client) {
                            socket.send(reply);
                        }
                        None
                    };
                    to_loop.push(ToLoop::Act(PendingAct {
                        ticket,
                        entity: entity.clone(),
                        act,
                    }));
                }
            }
        }
    }

    fn token_of(&self, entity: &EntityId) -> Option<String> {
        (self.pages.iter())
            .find(|(_, entry)| entry.entity == *entity)
            .map(|(token, _)| token.clone())
    }
}

/// See the module docs.
pub(super) struct Hub {
    state: Mutex<State>,
    /// What clients ask of the event loop: messages bound for a page, and
    /// acts waiting for a cursor.
    to_loop: Mutex<Sender<ToLoop>>,
    /// Gets the event loop to drain them.
    wake: Box<dyn Fn() + Send + Sync>,
}

impl Hub {
    pub(super) fn new(to_loop: Sender<ToLoop>, wake: Box<dyn Fn() + Send + Sync>) -> Self {
        Self {
            state: Mutex::new(State::default()),
            to_loop: Mutex::new(to_loop),
            wake,
        }
    }

    /// Every write under the lock is one insert, remove or proxy call that
    /// leaves the maps whole, so a panic elsewhere does not poison them.
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The token of `entity`'s endpoint, made with `fresh` on first asking,
    /// brought up to date with where the page is hosted and what it shows.
    pub(super) fn register(
        &self,
        entity: &EntityId,
        page: PageId,
        target: &Target,
        fresh: impl FnOnce() -> String,
    ) -> String {
        let mut state = self.state();
        if let Some(token) = state.token_of(entity)
            && let Some(entry) = state.pages.get_mut(&token)
        {
            entry.page = page;
            entry.proxy.set_target(&target.url, &target.title);
            return token;
        }
        let token = fresh();
        let entry = Entry {
            entity: entity.clone(),
            page,
            proxy: PageProxy::new(target.clone()),
        };
        state.pages.insert(token.clone(), entry);
        token
    }

    /// `generation` and `lastSnapshotGeneration`, as the CLI reads them.
    pub(super) fn generations(&self, entity: &EntityId) -> Value {
        let generation = (self.state().generations.get(entity).copied()).unwrap_or_default();
        json!({
            "generation": generation.current,
            "lastSnapshotGeneration": generation.snapshot,
        })
    }

    /// An agent read the page as it is now. Returns the generation it saw.
    pub(super) fn snapshot_seen(&self, entity: &EntityId) -> u64 {
        let mut state = self.state();
        let generation = state.generations.entry(entity.clone()).or_default();
        generation.snapshot = Some(generation.current);
        generation.current
    }

    /// The page shows another address: refs from before it are stale.
    pub(super) fn navigated(&self, entity: &EntityId, url: &str) {
        let mut state = self.state();
        state.generations.entry(entity.clone()).or_default().current += 1;
        if let Some(token) = state.token_of(entity)
            && let Some(entry) = state.pages.get_mut(&token)
        {
            let title = entry.proxy.target().title.clone();
            entry.proxy.set_target(url, &title);
        }
    }

    /// The page is no longer hosted: its endpoint goes, and its clients are
    /// hung up on.
    pub(super) fn page_closed(&self, entity: &EntityId) {
        let mut state = self.state();
        let gone: Vec<i32> = (state.scripts.iter())
            .filter(|(_, waiting)| waiting.entity == *entity)
            .map(|(id, _)| *id)
            .collect();
        let gone: Vec<Waiting> = gone
            .iter()
            .filter_map(|id| state.scripts.remove(id))
            .collect();
        for waiting in gone {
            (waiting.reply)(Response::error(503, format!("Page {entity} closed")));
        }
        state.generations.remove(entity);
        let Some(token) = state.token_of(entity) else {
            return;
        };
        state.pages.remove(&token);
        for (held, socket) in state.connections.values() {
            if *held == token {
                socket.close();
            }
        }
    }

    /// Notes `script`, about to be sent to `entity`'s page, and returns the
    /// id its message carries. `reply` is called with the answer.
    pub(super) fn expect_script(
        &self,
        entity: &EntityId,
        script: PageScript,
        reply: LaterReply,
    ) -> i32 {
        let mut state = self.state();
        let id = APP_ID_BASE.saturating_add(state.next_script);
        state.next_script = (state.next_script + 1) % (i32::MAX - APP_ID_BASE);
        let entity = entity.clone();
        state.scripts.insert(
            id,
            Waiting {
                entity,
                script,
                reply,
            },
        );
        id
    }

    /// The script `id` could not be sent: its caller hears why.
    pub(super) fn script_failed(&self, id: i32, reason: &str) {
        let waiting = self.state().scripts.remove(&id);
        if let Some(waiting) = waiting {
            (waiting.reply)(Response::error(500, format!("snapshot failed: {reason}")));
        }
    }

    /// The answer to one of the app's scripts, taken out of the waiting
    /// list, or `None` for a message that is not one.
    fn script_answer(&self, text: &str) -> Option<(Waiting, Value)> {
        let message: Value = serde_json::from_str(text).ok()?;
        let id = i32::try_from(message["id"].as_i64()?).ok()?;
        if id < APP_ID_BASE {
            return None;
        }
        Some((self.state().scripts.remove(&id)?, message))
    }

    /// A message from the page `page`'s devtools agent.
    pub(super) fn page_said(&self, page: PageId, text: &str) {
        if let Some((waiting, message)) = self.script_answer(text) {
            // Called outside the lock: the reply may take its time.
            (waiting.reply)(waiting.script.answer(&message));
            return;
        }
        let mut state = self.state();
        let Some(entry) = (state.pages.values_mut())
            .find(|entry| entry.page == page && entry.proxy.has_clients())
        else {
            return;
        };
        let out = entry.proxy.from_page(text);
        let entity = entry.entity.clone();
        // A page only ever answers its clients.
        state.deliver((&entity, page), out, &mut Vec::new());
    }

    /// The page would not take the message `id`: its client hears why.
    pub(super) fn refuse(&self, page: PageId, id: i32, reason: &str) {
        let mut state = self.state();
        let Some(entry) = state.pages.values_mut().find(|entry| entry.page == page) else {
            return;
        };
        let out = entry.proxy.refuse(id, reason).into_iter().collect();
        let entity = entry.entity.clone();
        state.deliver((&entity, page), out, &mut Vec::new());
    }

    /// The cursor of the act `ticket` has arrived: its client is told to
    /// go on. A client that has gone is owed nothing.
    pub(super) fn go(&self, ticket: u64) {
        let mut state = self.state();
        if let Some((client, reply)) = state.acts.remove(&ticket)
            && let Some((_, socket)) = state.connections.get(&client)
        {
            socket.send(reply);
        }
    }

    /// How many endpoints and how many connected clients there are.
    pub(super) fn counts(&self) -> (usize, usize) {
        let state = self.state();
        (state.pages.len(), state.connections.len())
    }
}

impl WsHandler for Hub {
    fn open(&self, path: &str, out: WsSender) -> Option<u64> {
        let token = path.strip_prefix(PATH)?;
        let mut state = self.state();
        state.next_connection += 1;
        let connection = state.next_connection;
        state.pages.get_mut(token)?.proxy.connect(connection);
        (state.connections).insert(connection, (token.to_owned(), out));
        Some(connection)
    }

    fn message(&self, connection: u64, text: String) {
        let mut to_loop = Vec::new();
        {
            let mut state = self.state();
            let Some((token, _)) = state.connections.get(&connection) else {
                return;
            };
            let token = token.clone();
            let Some(entry) = state.pages.get_mut(&token) else {
                return;
            };
            let (entity, page) = (entry.entity.clone(), entry.page);
            let out = entry.proxy.from_client(connection, &text);
            state.deliver((&entity, page), out, &mut to_loop);
        }
        if to_loop.is_empty() {
            return;
        }
        let sender = self.to_loop.lock().unwrap_or_else(PoisonError::into_inner);
        for message in to_loop {
            // The loop has gone only while the app is closing.
            let _ = sender.send(message);
        }
        drop(sender);
        (self.wake)();
    }

    fn closed(&self, connection: u64) {
        let mut state = self.state();
        state.acts.retain(|_, (client, _)| *client != connection);
        if let Some((token, _)) = state.connections.remove(&connection)
            && let Some(entry) = state.pages.get_mut(&token)
        {
            entry.proxy.disconnect(connection);
        }
    }
}
