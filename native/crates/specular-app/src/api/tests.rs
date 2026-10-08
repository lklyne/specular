//! The server over a real socket: a client thread speaks HTTP to an
//! ephemeral port while the test's own thread plays the event loop, with a
//! scripted app where the shell would be.

use std::io::{Read as _, Write as _};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};
use specular_api::{Host, Screenshot};
use specular_interact::{ApiOutcome, App, Effect, Event};
use specular_testkit::{TestApp, assert_doc_snapshot};

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
fn a_session_over_http_builds_a_canvas() {
    let mut server = Loopback::new();
    // What `specular add note` does: ask where it goes, then apply.
    let spot = server.ok(
        "POST",
        "/layout/batch-placement",
        &json!({ "items": [{ "width": 200, "height": 200, "insetX": 0, "insetY": 0 }], "anchor": "selection_or_empty_region" }),
    );
    let at = &spot["positions"][0];
    assert_eq!(at, &json!({ "canvasX": 80, "canvasY": 80 }));
    let note = server.ok(
        "POST",
        "/canvas/apply",
        &json!({ "entities": [{ "kind": "text", "text": "ship it", "canvasX": at["canvasX"], "canvasY": at["canvasY"], "color": "4" }] }),
    );
    let note = note["created"][0].clone();
    // `specular add page <url> --at 400,80 --preset 1`.
    let page = server.ok(
        "POST",
        "/canvas/apply",
        &json!({ "entities": [{ "kind": "page", "url": "http://localhost:4321/garden", "presetIndex": 1, "canvasX": 400, "canvasY": 80 }] }),
    )["created"][0]
        .clone();
    // `specular link`, `specular update --text`, `specular annotate-selection`.
    let edge = server.ok(
        "POST",
        "/canvas/apply",
        &json!({ "edges": [{ "fromEntityId": note, "toEntityId": page, "kind": "connection", "label": "about" }] }),
    )["edges"][0]
        .clone();
    server.ok(
        "POST",
        "/canvas/apply",
        &json!({ "entities": [{ "id": note, "text": "ship it today" }] }),
    );
    let comment = server.ok(
        "POST",
        "/selection/annotate",
        &json!({ "text": "check the hero", "entityIds": [page] }),
    );
    assert_eq!(
        comment["selectionTarget"]["url"],
        "http://localhost:4321/garden"
    );

    // The read sees what the writes made, and the edge between them.
    let canvas = server.ok("GET", "/canvas", &json!({}));
    let ids: Vec<&Value> = canvas["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| &node["id"])
        .collect();
    assert_eq!(ids, [&note, &page]);
    assert_eq!(canvas["edges"][0]["id"], edge);
    assert_eq!(
        canvas["appState"]["activeTab"],
        json!({ "id": "tab_1", "name": "Canvas 1" })
    );
    let listed = server.ok("GET", "/annotations?status=unresolved", &json!({}));
    assert_eq!(listed["annotations"][0]["text"], "check the hero");

    // A refusal reaches the caller as the CLI expects it: a status and `error`.
    assert_eq!(
        server
            .call(
                "POST",
                "/canvas/apply",
                &json!({ "entities": [{ "kind": "page", "url": "/garden" }] })
            )
            .0,
        400
    );
    assert_doc_snapshot!("a_session_over_http_builds_a_canvas", server.seat.0);
    // Each write was one step, so undo walks all the way back.
    server.seat.0.assert_undo_returns_to_start();
}
