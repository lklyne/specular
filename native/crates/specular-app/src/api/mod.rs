//! The HTTP API's home in the shell: a server on its own thread, bridged to
//! the event loop with a channel and a wake.
//!
//! ```text
//! HTTP thread:  request -> Job { request, reply } -> channel, then wake()
//! event loop:   ApiHost::serve(host) -> Api::answer -> reply
//! HTTP thread:  reply -> response
//! ```
//!
//! The routes are `specular-api`'s and know nothing of sockets. This module
//! is the transport: the port, the secret and the discovery file the CLI
//! finds the app by, all as the Electron app has them.

mod discovery;
mod server;
#[cfg(test)]
mod tests;

use std::sync::mpsc::{self, Receiver};

use specular_api::{Api, Host};

use self::discovery::Discovery;
pub(crate) use self::discovery::new_secret;
use self::server::{Job, Server};

/// The API as the event loop holds it: the routes' state, the requests
/// waiting to be answered, and the server feeding them.
pub(crate) struct ApiHost {
    api: Api,
    inbox: Receiver<Job>,
    server: Server,
    /// The discovery file, removed when the app goes.
    discovery: Option<Discovery>,
}

impl ApiHost {
    /// Starts the server and writes the discovery file. `wake` is called on
    /// the server's thread after each request is queued; it must get the
    /// event loop to call [`serve`](Self::serve).
    pub(crate) fn start(id_seed: u64, wake: impl Fn() + Send + 'static) -> anyhow::Result<Self> {
        let secret = discovery::new_secret();
        let bound = discovery::bind(discovery::preferred_port())?;
        let (jobs, inbox) = mpsc::channel();
        let port = bound.port;
        let server = Server::spawn(bound.listener, secret.clone(), jobs, wake)?;
        let discovery = Discovery::write(bound.file, port, &secret);
        tracing::info!(port, "API listening on http://127.0.0.1:{port}");
        Ok(Self {
            api: Api::new(id_seed),
            inbox,
            server,
            discovery,
        })
    }

    /// A server on a port the system picks, with no discovery file.
    #[cfg(test)]
    fn start_for_test(secret: &str, wake: impl Fn() + Send + 'static) -> anyhow::Result<Self> {
        let listener = tiny_http::Server::http("127.0.0.1:0").map_err(anyhow::Error::from_boxed)?;
        let (jobs, inbox) = mpsc::channel();
        Ok(Self {
            api: Api::new(0),
            inbox,
            server: Server::spawn(listener, secret.to_owned(), jobs, wake)?,
            discovery: None,
        })
    }

    /// The port the server is listening on.
    #[cfg(test)]
    fn port(&self) -> u16 {
        self.server.port()
    }

    /// Answers every request that has arrived, against `host`.
    pub(crate) fn serve(&mut self, host: &mut impl Host) {
        while let Ok(job) = self.inbox.try_recv() {
            let response = self.api.answer(host, &job.request);
            // The server thread gives up on a request that takes too long.
            let _ = job.reply.send(response);
        }
    }
}

impl Drop for ApiHost {
    fn drop(&mut self) {
        self.server.stop();
        if let Some(discovery) = self.discovery.take() {
            discovery.remove();
        }
    }
}
