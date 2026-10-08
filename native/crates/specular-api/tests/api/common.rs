//! A scripted app behind the API: what the shell is to the window, for
//! tests.
use std::collections::HashMap;

use serde_json::{Value, json};
use specular_api::{Api, CdpAsk, Host, Method, Request, Response, Screenshot, ShotArea};
use specular_interact::{ApiOutcome, App, DroppedFile, Effect, Event};
use specular_testkit::TestApp;

/// A [`TestApp`] answering API requests.
pub(crate) struct Scripted {
    pub(crate) app: TestApp,
    pub(crate) api: Api,
    /// The effects of the latest write, the reply left out.
    pub(crate) effects: Vec<Effect>,
    /// Every picture asked for that a test host could stand in for.
    pub(crate) shots: Vec<Screenshot>,
    /// What the host finds on disk.
    pub(crate) disk: Disk,
}

/// The disk the host answers from: the files it knows by the path a
/// request names, and the names in the space folder.
#[derive(Default)]
pub(crate) struct Disk {
    pub(crate) files: HashMap<String, DroppedFile>,
    pub(crate) entries: Vec<String>,
}

struct Seat<'a> {
    app: &'a mut TestApp,
    effects: &'a mut Vec<Effect>,
    shots: &'a mut Vec<Screenshot>,
    disk: &'a Disk,
}

impl Host for Seat<'_> {
    fn app(&self) -> &App {
        self.app.app()
    }

    fn run(&mut self, event: Event) -> Option<ApiOutcome> {
        self.app.take_effects();
        self.app.send(event);
        let mut outcome = None;
        self.effects.clear();
        for effect in self.app.take_effects() {
            match effect {
                Effect::ApiReply { outcome: found, .. } => outcome = Some(found),
                other => self.effects.push(other),
            }
        }
        outcome
    }

    /// The window cannot be drawn with no renderer. A page or a region is
    /// answered with a stand-in and kept, so a test can read what was asked.
    fn screenshot(&mut self, shot: &Screenshot) -> Result<Value, String> {
        if shot.area == ShotArea::Window {
            return Err("a test has no renderer".to_owned());
        }
        self.shots.push(shot.clone());
        Ok(json!({ "mimeType": "image/png", "width": 2, "height": 1, "base64": "cGl4ZWxz" }))
    }

    fn cdp(&mut self, ask: &CdpAsk) -> Result<Value, Response> {
        Ok(match ask {
            CdpAsk::Target { page, .. } => json!({
                "webSocketDebuggerUrl": format!("ws://127.0.0.1:1/cdp/page/token-{page}"),
                "generation": 0,
                "lastSnapshotGeneration": null,
            }),
            CdpAsk::SnapshotSeen { .. } => json!({ "ok": true, "generation": 0 }),
        })
    }

    fn inspect_file(&self, path: &str) -> Option<DroppedFile> {
        self.disk.files.get(path).cloned()
    }

    fn space_entries(&self) -> Vec<String> {
        self.disk.entries.clone()
    }
}

impl Scripted {
    pub(crate) fn new(app: TestApp) -> Self {
        Self {
            app,
            api: Api::new(0),
            effects: Vec::new(),
            shots: Vec::new(),
            disk: Disk::default(),
        }
    }

    pub(crate) fn empty() -> Self {
        let mut app = TestApp::empty();
        app.viewport((1000.0, 800.0));
        Self::new(app)
    }

    pub(crate) fn call(&mut self, request: &Request) -> Response {
        let mut seat = Seat {
            app: &mut self.app,
            effects: &mut self.effects,
            shots: &mut self.shots,
            disk: &self.disk,
        };
        self.api.answer(&mut seat, request)
    }

    pub(crate) fn get(&mut self, target: &str) -> Response {
        self.call(&Request::get(target))
    }

    pub(crate) fn post(&mut self, target: &str, body: Value) -> Response {
        self.call(&Request::post(target, body))
    }

    pub(crate) fn delete(&mut self, target: &str) -> Response {
        self.call(&Request::new(Method::Delete, target, json!({})))
    }

    /// Posts a patch, which must be accepted, and returns its answer.
    #[track_caller]
    pub(crate) fn apply(&mut self, patch: Value) -> Value {
        let response = self.post("/canvas/apply", patch);
        assert_eq!(response.status, 200, "{}", response.body);
        response.body
    }

    /// `GET /canvas`.
    #[track_caller]
    pub(crate) fn canvas(&mut self) -> Value {
        let response = self.get("/canvas");
        assert_eq!(response.status, 200, "{}", response.body);
        response.body
    }

    /// The node `id` names in `GET /canvas`, or `null`.
    pub(crate) fn node(&mut self, id: &Value) -> Value {
        let canvas = self.canvas();
        let nodes = canvas["nodes"].as_array().cloned().unwrap_or_default();
        (nodes.into_iter())
            .find(|node| node["id"] == *id)
            .unwrap_or(Value::Null)
    }

    pub(crate) fn undo(&mut self) {
        let response = self.post("/history/undo", json!({}));
        assert_eq!(response.body["ok"], true, "{}", response.body);
    }
}
