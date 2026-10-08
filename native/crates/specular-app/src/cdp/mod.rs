//! A CDP websocket for each page, speaking for that page alone.
//!
//! ```text
//! agent-browser --ws--> socket --> Hub --ToPage--> event loop --> page's devtools channel
//! agent-browser <--ws-- socket <-- Hub <------- devtools sink <-- page's devtools channel
//! ```
//!
//! `GET /pages/<id>/cdp-target` answers with
//! `ws://127.0.0.1:<port>/cdp/page/<token>`. The token names one page, and
//! what is said on that socket goes down that page's own devtools channel
//! ([`PageSource::devtools_send`]), so there is no shared debugging port to
//! pick the wrong page on. The routing is `specular_api::cdp`; this module
//! is the transport: [`socket`] and the framing under it, and [`hub`],
//! where the threads meet.

mod frame;
mod handshake;
mod hub;
pub(crate) mod socket;
#[cfg(test)]
pub(crate) mod test_client;
#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};

use serde_json::{Value, json};
use specular_api::cdp::Target;
use specular_core::{DevtoolsSink, PageId, PageSource};
use specular_doc::EntityId;

use self::hub::{Hub, ToPage};
use self::socket::WsServer;

/// The endpoints as the event loop holds them.
pub(crate) struct CdpHost {
    hub: Arc<Hub>,
    inbox: Receiver<ToPage>,
    server: WsServer,
}

impl CdpHost {
    /// Starts the websocket server. `wake` is called on a connection's
    /// thread when a client's message is waiting; it must get the event
    /// loop to call [`serve`](Self::serve).
    pub(crate) fn start(wake: impl Fn() + Send + Sync + 'static) -> std::io::Result<Self> {
        let (to_loop, inbox) = mpsc::channel();
        let hub = Arc::new(Hub::new(to_loop, Box::new(wake)));
        let server = WsServer::spawn(Arc::clone(&hub) as Arc<dyn socket::WsHandler>)?;
        tracing::info!(port = server.port(), "page CDP endpoints listening");
        Ok(Self { hub, inbox, server })
    }

    /// What the page backend hands every page's devtools messages to.
    pub(crate) fn sink(&self) -> DevtoolsSink {
        let hub = Arc::clone(&self.hub);
        Arc::new(move |page, message| hub.page_said(page, message))
    }

    /// The answer to `GET /pages/<id>/cdp-target` for the page entity
    /// `entity`, hosted as `page`.
    pub(crate) fn target(&self, entity: &EntityId, page: PageId, target: &Target) -> Value {
        let token = (self.hub).register(entity, page, target, crate::api::new_secret);
        let mut body = self.hub.generations(entity);
        let port = self.server.port();
        body["webSocketDebuggerUrl"] = json!(format!("ws://127.0.0.1:{port}{}{token}", hub::PATH));
        body
    }

    /// An agent read the page; see [`Hub::snapshot_seen`].
    pub(crate) fn snapshot_seen(&self, entity: &EntityId) -> u64 {
        self.hub.snapshot_seen(entity)
    }

    /// The page shows another address.
    pub(crate) fn navigated(&self, entity: &EntityId, url: &str) {
        self.hub.navigated(entity, url);
    }

    /// The page is no longer hosted.
    pub(crate) fn page_closed(&self, entity: &EntityId) {
        self.hub.page_closed(entity);
    }

    /// Sends every waiting client message to its page.
    pub(crate) fn serve(&mut self, source: &mut dyn PageSource) {
        while let Ok(ToPage { page, id, message }) = self.inbox.try_recv() {
            if let Err(error) = source.devtools_send(page, &message) {
                self.hub.refuse(page, id, &error.to_string());
            }
        }
    }

    /// How many endpoints and connected clients there are, for the log.
    pub(crate) fn counts(&self) -> (usize, usize) {
        self.hub.counts()
    }
}

impl Drop for CdpHost {
    fn drop(&mut self) {
        self.server.stop();
    }
}
