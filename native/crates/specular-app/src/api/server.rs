//! The HTTP thread: reads a request, checks the secret, hands it to the
//! event loop and writes back what comes.

use std::io::Read as _;
use std::sync::Arc;
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::Context as _;
use serde_json::{Value, json};
use specular_api::{Method, Request, Response, SECRET_HEADER, TAB_HEADER, VERSION};

/// How long the event loop has to answer before the caller gets a `504`.
const REPLY_TIMEOUT: Duration = Duration::from_secs(30);
/// The largest body read. A patch for a whole canvas is far smaller.
const MAX_BODY: u64 = 64 * 1024 * 1024;

/// One request on its way to the event loop, with where its answer goes.
pub(super) struct Job {
    pub(super) request: Request,
    pub(super) reply: Sender<Response>,
}

/// The running server.
pub(super) struct Server {
    listener: Arc<tiny_http::Server>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    /// Serves `listener` on a new thread. Each authorized request is sent
    /// to `jobs`, then `wake` is called.
    pub(super) fn spawn(
        listener: tiny_http::Server,
        secret: String,
        jobs: Sender<Job>,
        wake: impl Fn() + Send + 'static,
    ) -> anyhow::Result<Self> {
        let listener = Arc::new(listener);
        let serving = Arc::clone(&listener);
        let thread = std::thread::Builder::new()
            .name("specular-api".to_owned())
            .spawn(move || {
                for request in serving.incoming_requests() {
                    handle(request, &secret, &jobs, &wake);
                }
            })
            .context("starting the API thread")?;
        Ok(Self {
            listener,
            thread: Some(thread),
        })
    }

    #[cfg(test)]
    pub(super) fn port(&self) -> u16 {
        (self.listener.server_addr().to_ip()).map_or(0, |address| address.port())
    }

    /// Stops accepting requests and waits for the thread to end.
    pub(super) fn stop(&mut self) {
        self.listener.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn header<'r>(request: &'r tiny_http::Request, name: &str) -> Option<&'r str> {
    (request.headers().iter())
        .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str())
}

/// The request's JSON body. An empty one is `{}`, as the Electron server
/// reads it.
fn body(request: &mut tiny_http::Request) -> Result<Value, String> {
    let mut text = String::new();
    (request.as_reader().take(MAX_BODY))
        .read_to_string(&mut text)
        .map_err(|error| error.to_string())?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&text).map_err(|error| error.to_string())
}

/// What the request gets back, short of an answer from the event loop.
fn answer(
    request: &mut tiny_http::Request,
    secret: &str,
    jobs: &Sender<Job>,
    wake: &impl Fn(),
) -> Response {
    let target = request.url().to_owned();
    let Some(method) = Method::parse(request.method().as_str()) else {
        let method = request.method().as_str().to_owned();
        return Response::error(404, format!("Unknown route: {method} {target}"));
    };
    // The one route with no secret: how a caller tells a Specular is here.
    if method == Method::Get && target == "/health" {
        return Response::ok(json!({ "version": VERSION }));
    }
    if header(request, SECRET_HEADER) != Some(secret) {
        return Response::error(401, "Unauthorized");
    }
    let body = match method {
        Method::Get => json!({}),
        Method::Post | Method::Delete => match body(request) {
            Ok(body) => body,
            Err(error) => return Response::error(400, error),
        },
    };
    let tab = header(request, TAB_HEADER).map(specular_api::percent_decode);
    let request = Request {
        tab,
        ..Request::new(method, &target, body)
    };
    let (reply, answered) = mpsc::channel();
    if jobs.send(Job { request, reply }).is_err() {
        return Response::error(503, "the app is shutting down");
    }
    wake();
    answered
        .recv_timeout(REPLY_TIMEOUT)
        .unwrap_or_else(|_| Response::error(504, "the app did not answer in time"))
}

fn handle(mut request: tiny_http::Request, secret: &str, jobs: &Sender<Job>, wake: &impl Fn()) {
    let response = answer(&mut request, secret, jobs, wake);
    let mut reply = tiny_http::Response::from_string(response.body.to_string())
        .with_status_code(response.status);
    if let Ok(json) = tiny_http::Header::from_bytes("Content-Type", "application/json") {
        reply.add_header(json);
    }
    if let Err(error) = request.respond(reply) {
        tracing::debug!("API response not sent: {error}");
    }
}
