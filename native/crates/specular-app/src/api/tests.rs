//! The server over a real socket: a client thread speaks HTTP to an
//! ephemeral port while the test's own thread plays the event loop, with a
//! scripted app where the shell would be.

use std::io::{Read as _, Write as _};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};
use specular_api::{CdpAsk, Host, LaterReply, PageScript, Response, Screenshot};
use specular_interact::{ApiOutcome, App, Effect, Event};
use specular_testkit::TestApp;

use super::ApiHost;

const SECRET: &str = "test-secret";

/// The scripted app as the API's host.
struct Seat(TestApp);

impl Host for Seat {
    fn app(&self) -> &App {
        self.0.app()
    }

    fn run(&mut self, event: Event) -> Option<ApiOutcome> {
        self.0.take_effects();
        self.0.send(event);
        (self.0.take_effects().into_iter()).find_map(|effect| match effect {
            Effect::ApiReply { outcome, .. } => Some(outcome),
            _ => None,
        })
    }

    fn screenshot(&mut self, _shot: &Screenshot) -> Result<Value, String> {
        Err("a test has no renderer".to_owned())
    }

    fn cdp(&mut self, _ask: &CdpAsk) -> Result<Value, Response> {
        Err(Response::error(503, "a test has no page backend"))
    }

    /// The page answers from another thread a moment later, as a real one
    /// answers from inside the backend's pump.
    fn page_script(&mut self, script: PageScript, reply: LaterReply) {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            let value = json!({ "url": "https://example.com/", "title": "Example", "nodes": [] });
            reply(script.answer(&json!({ "id": 1, "result": { "result": { "value": value } } })));
        });
    }
}

/// A server, the app behind it, and the wakes the server sends.
struct Loopback {
    host: ApiHost,
    seat: Seat,
    wakes: Receiver<()>,
}

/// One HTTP exchange, written by hand so the test needs no client crate.
fn exchange(port: u16, method: &str, target: &str, headers: &str, body: &str) -> (u16, Value) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let request = format!(
        "{method} {target} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n{headers}\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut answer = String::new();
    stream.read_to_string(&mut answer).unwrap();
    let (head, body) = answer.split_once("\r\n\r\n").unwrap();
    let status = head.split(' ').nth(1).unwrap().parse().unwrap();
    assert!(
        head.to_ascii_lowercase()
            .contains("content-type: application/json"),
        "{head}"
    );
    (status, serde_json::from_str(body).unwrap())
}

impl Loopback {
    fn new() -> Self {
        let (wake, wakes) = mpsc::channel();
        let host = ApiHost::start_for_test(SECRET, move || {
            let _ = wake.send(());
        })
        .unwrap();
        let mut app = TestApp::empty();
        app.viewport((1000.0, 800.0));
        Self {
            host,
            seat: Seat(app),
            wakes,
        }
    }

    /// Sends one request from a client thread and turns the loop until it
    /// is answered.
    fn send(&mut self, method: &str, target: &str, headers: &str, body: &str) -> (u16, Value) {
        let port = self.host.port();
        let (method, target) = (method.to_owned(), target.to_owned());
        let (headers, body) = (headers.to_owned(), body.to_owned());
        let client = std::thread::spawn(move || exchange(port, &method, &target, &headers, &body));
        while !client.is_finished() {
            if self.wakes.recv_timeout(Duration::from_millis(5)).is_ok() {
                self.host.serve(&mut self.seat);
            }
        }
        client.join().unwrap()
    }

    fn call(&mut self, method: &str, target: &str, body: &Value) -> (u16, Value) {
        let secret = format!("x-specular-secret: {SECRET}\r\n");
        self.send(method, target, &secret, &body.to_string())
    }

    #[track_caller]
    fn ok(&mut self, method: &str, target: &str, body: &Value) -> Value {
        let (status, body) = self.call(method, target, body);
        assert_eq!(status, 200, "{target}: {body}");
        body
    }
}

#[test]
fn a_request_needs_the_secret_and_a_json_body() {
    let mut server = Loopback::new();
    // Health is how a caller finds a Specular, so it asks for no secret.
    assert_eq!(
        server.send("GET", "/health", "", ""),
        (200, json!({ "version": "1" }))
    );
    assert_eq!(
        server.send("GET", "/canvas", "", ""),
        (401, json!({ "error": "Unauthorized" }))
    );
    let wrong = "x-specular-secret: guess\r\n";
    assert_eq!(server.send("GET", "/canvas", wrong, "").0, 401);

    let secret = format!("x-specular-secret: {SECRET}\r\n");
    let (status, body) = server.send("POST", "/canvas/apply", &secret, "{not json");
    assert_eq!(status, 400);
    assert!(
        body["error"]
            .as_str()
            .is_some_and(|error| error.contains("line 1")),
        "{body}"
    );
    assert_eq!(
        server.send("PUT", "/canvas", &secret, ""),
        (404, json!({ "error": "Unknown route: PUT /canvas" }))
    );
    // An empty body is `{}`.
    assert_eq!(
        server.send("POST", "/selection/deselect", &secret, "").0,
        200
    );
    assert_eq!(
        server.call("POST", "/window/screenshot", &json!({})),
        (500, json!({ "error": "a test has no renderer" }))
    );
}

#[test]
fn a_tab_ref_arrives_decoded() {
    let mut server = Loopback::new();
    let tabbed = |tab: &str| format!("x-specular-secret: {SECRET}\r\nx-specular-tab: {tab}\r\n");
    assert_eq!(
        server.send("GET", "/canvas", &tabbed("Canvas%201"), "").0,
        200
    );
    assert_eq!(
        server.send("GET", "/canvas", &tabbed("sync%20roads"), ""),
        (
            400,
            json!({ "error": "unknown tab 'sync roads' \u{2014} available: tab_1 (Canvas 1)" })
        )
    );
}

#[test]
fn a_page_snapshot_is_answered_once_the_page_has_answered() {
    let mut loopback = Loopback::new();
    let made = loopback.ok(
        "POST",
        "/canvas/apply",
        &json!({ "entities": [{ "kind": "page", "url": "https://example.com/" }] }),
    );
    let page = made["created"][0].clone();
    let body = loopback.ok("POST", "/pages/agent-snapshot", &json!({ "pageId": page }));
    assert_eq!(
        body["snapshot"],
        json!({ "pageId": page, "url": "https://example.com/", "title": "Example", "nodes": [] })
    );
}
