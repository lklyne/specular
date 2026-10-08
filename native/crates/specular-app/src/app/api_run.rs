//! The shell as the HTTP API's host: starting the server, answering what it
//! queues, and drawing the canvas for a screenshot.

use super::runtime::{Runtime, ShellWindow, unix_ms};
use crate::api::ApiHost;
use crate::cdp::CdpHost;
use serde_json::{Value, json};
use specular_api::{CdpAsk, Host, Response, Screenshot};
use specular_interact::{ApiOutcome, App, DroppedFile, Event};

/// What another thread can ask the event loop to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellEvent {
    /// The API has requests waiting.
    Api,
}

impl<W: ShellWindow> Runtime<W> {
    /// Starts the API and the page CDP endpoints. `wake` is called on a
    /// server thread when a request is waiting, and the shell then calls
    /// [`serve_api`](Self::serve_api) on its own. A server that cannot
    /// start is logged and the app runs on without it.
    pub fn start_api(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        let wake = std::sync::Arc::new(wake);
        let for_api = std::sync::Arc::clone(&wake);
        match ApiHost::start(unix_ms(), move || for_api()) {
            Ok(api) => self.api = Some(api),
            Err(error) => tracing::warn!("the API is off: {error:#}"),
        }
        match CdpHost::start(move || wake()) {
            Ok(cdp) => {
                self.source.set_devtools_sink(Some(cdp.sink()));
                self.cdp = Some(cdp);
            }
            Err(error) => tracing::warn!("the page CDP endpoints are off: {error}"),
        }
    }

    /// Answers the requests the API has queued, and sends on what CDP
    /// clients have said to their pages.
    pub fn serve_api(&mut self) {
        if self.closing {
            return;
        }
        // Taken out for the call: answering borrows the whole shell.
        if let Some(mut api) = self.api.take() {
            api.serve(self);
            self.api = Some(api);
        }
        if let Some(cdp) = self.cdp.as_mut() {
            cdp.serve(self.source.as_mut());
            // A backend with no loop of its own answers when pumped.
            self.source.pump();
        }
    }
}

impl<W: ShellWindow> Host for Runtime<W> {
    fn app(&self) -> &App {
        &self.app
    }

    fn run(&mut self, event: Event) -> Option<ApiOutcome> {
        self.api_outcome = None;
        self.dispatch(event);
        self.api_outcome.take()
    }

    fn screenshot(&mut self, shot: &Screenshot) -> Result<Value, String> {
        self.shoot(shot)
    }

    fn cdp(&mut self, ask: &CdpAsk) -> Result<Value, Response> {
        let off = || {
            Response::error(
                503,
                "the page CDP endpoints did not start; see the app's log",
            )
        };
        let cdp = self.cdp.as_ref().ok_or_else(off)?;
        match ask {
            CdpAsk::Target { page, target } => {
                let host = self.hosts.get(page).ok_or_else(|| {
                    Response::error(503, format!("Page {page} is not hosted yet"))
                })?;
                let body = cdp.target(page, host.page, target);
                let (endpoints, clients) = cdp.counts();
                tracing::debug!(%page, endpoints, clients, "CDP target resolved");
                Ok(body)
            }
            CdpAsk::SnapshotSeen { page } => {
                Ok(json!({ "ok": true, "generation": cdp.snapshot_seen(page) }))
            }
        }
    }

    fn inspect_file(&self, path: &str) -> Option<DroppedFile> {
        self.inspect_space_file(path)
    }

    fn space_entries(&self) -> Vec<String> {
        self.list_space_folder()
    }
}
