//! A scripted app behind the API: what the shell is to the window, for
//! tests.
use serde_json::{Value, json};
use specular_api::{Api, Host, Method, Request, Response, Screenshot, Tab};
use specular_interact::{ApiOutcome, App, Effect, Event};
use specular_testkit::TestApp;

/// A [`TestApp`] answering API requests.
pub(crate) struct Scripted {
    pub(crate) app: TestApp,
    pub(crate) api: Api,
    /// The effects of the latest write, the reply left out.
    pub(crate) effects: Vec<Effect>,
}

struct Seat<'a> {
    app: &'a mut TestApp,
    effects: &'a mut Vec<Effect>,
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

    fn screenshot(&mut self, _shot: &Screenshot) -> Result<Value, String> {
        Err("a test has no renderer".to_owned())
    }
}

impl Scripted {
    pub(crate) fn new(app: TestApp) -> Self {
        let tab = Tab {
            id: "tab_1".to_owned(),
            name: "Canvas".to_owned(),
        };
        Self {
            app,
            api: Api::new(tab, 0),
            effects: Vec::new(),
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
