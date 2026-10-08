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
use specular_api::cdp::{Out, PageProxy, Target};
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
}

impl State {
    fn deliver(&self, page: PageId, out: Vec<Out>, to_page: &mut Vec<ToPage>) {
        for out in out {
            match out {
                Out::ToClient(client, text) => {
                    if let Some((_, socket)) = self.connections.get(&client) {
                        socket.send(text);
                    }
                }
                Out::ToPage { id, message } => to_page.push(ToPage { page, id, message }),
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
    /// Client messages bound for a page, for the event loop.
    to_loop: Mutex<Sender<ToPage>>,
    /// Gets the event loop to drain them.
    wake: Box<dyn Fn() + Send + Sync>,
}

impl Hub {
    pub(super) fn new(to_loop: Sender<ToPage>, wake: Box<dyn Fn() + Send + Sync>) -> Self {
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

    /// A message from the page `page`'s devtools agent.
    pub(super) fn page_said(&self, page: PageId, text: &str) {
        let mut state = self.state();
        let Some(entry) = (state.pages.values_mut())
            .find(|entry| entry.page == page && entry.proxy.has_clients())
        else {
            return;
        };
        let out = entry.proxy.from_page(text);
        // A page only ever answers its clients.
        state.deliver(page, out, &mut Vec::new());
    }

    /// The page would not take the message `id`: its client hears why.
    pub(super) fn refuse(&self, page: PageId, id: i32, reason: &str) {
        let mut state = self.state();
        let Some(entry) = state.pages.values_mut().find(|entry| entry.page == page) else {
            return;
        };
        let out = entry.proxy.refuse(id, reason).into_iter().collect();
        state.deliver(page, out, &mut Vec::new());
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
        let mut to_page = Vec::new();
        {
            let mut state = self.state();
            let Some((token, _)) = state.connections.get(&connection) else {
                return;
            };
            let token = token.clone();
            let Some(entry) = state.pages.get_mut(&token) else {
                return;
            };
            let page = entry.page;
            let out = entry.proxy.from_client(connection, &text);
            state.deliver(page, out, &mut to_page);
        }
        if to_page.is_empty() {
            return;
        }
        let to_loop = self.to_loop.lock().unwrap_or_else(PoisonError::into_inner);
        for message in to_page {
            // The loop has gone only while the app is closing.
            let _ = to_loop.send(message);
        }
        drop(to_loop);
        (self.wake)();
    }

    fn closed(&self, connection: u64) {
        let mut state = self.state();
        if let Some((token, _)) = state.connections.remove(&connection)
            && let Some(entry) = state.pages.get_mut(&token)
        {
            entry.proxy.disconnect(connection);
        }
    }
}
