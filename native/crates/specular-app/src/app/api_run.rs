//! The shell as the HTTP API's host: starting the server, answering what it
//! queues, and drawing the canvas for a screenshot.

use super::runtime::{Runtime, ShellWindow, unix_ms};
use crate::api::ApiHost;
use base64::Engine as _;
use serde_json::{Value, json};
use specular_api::{Host, Screenshot};
use specular_doc::EntityId;
use specular_interact::{ApiOutcome, App, Event};

/// What another thread can ask the event loop to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShellEvent {
    /// The API has requests waiting.
    Api,
}

impl<W: ShellWindow> Runtime<W> {
    /// Starts the API. `wake` is called on the server's thread when a
    /// request is waiting, and the shell then calls
    /// [`serve_api`](Self::serve_api) on its own. A server that cannot
    /// start is logged and the app runs on without one.
    pub fn start_api(&mut self, wake: impl Fn() + Send + 'static) {
        let started = ApiHost::start(unix_ms(), wake);
        match started {
            Ok(api) => self.api = Some(api),
            Err(error) => tracing::warn!("the API is off: {error:#}"),
        }
    }

    /// Answers the requests the API has queued.
    pub fn serve_api(&mut self) {
        if self.closing {
            return;
        }
        // Taken out for the call: answering borrows the whole shell.
        if let Some(mut api) = self.api.take() {
            api.serve(self);
            self.api = Some(api);
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
        let gpu = self.gpu.as_mut().ok_or("the window is not open yet")?;
        let scene = specular_scene::view(&self.app, gpu.logical_viewport());
        let hosts = &self.hosts;
        let page_of = |entity: &EntityId| hosts.get(entity).map(|host| host.page);
        let (png, width, height) = gpu
            .capture(self.app.session().camera, &scene, &page_of)
            .map_err(|error| format!("{error:#}"))?;
        let mut body = json!({ "mimeType": "image/png", "width": width, "height": height });
        match &shot.path {
            Some(path) => {
                std::fs::write(path, png)
                    .map_err(|error| format!("writing {}: {error}", path.display()))?;
                body["path"] = json!(path);
            }
            None => {
                body["base64"] = json!(base64::engine::general_purpose::STANDARD.encode(png));
            }
        }
        Ok(body)
    }
}
