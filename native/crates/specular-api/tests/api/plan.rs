//! `Api::plan` on its own: a request and an app in, an answer or an event
//! out, and nothing changed.
#![expect(
    clippy::panic,
    reason = "a helper panics to fail the test it is called from"
)]

use serde_json::{Value, json};
use specular_api::{Api, Method, Plan, Request, Response};
use specular_doc::Rect;
use specular_interact::{ApiCall, ApiRun, Event};
use specular_testkit::{TestApp, document, group, inside, sticky};

fn api() -> Api {
    Api::new(0)
}

fn notes() -> TestApp {
    let mut app = TestApp::from_document(document([
        sticky("a", Rect::new(0.0, 0.0, 200.0, 200.0), "one"),
        sticky("b", Rect::new(300.0, 0.0, 200.0, 200.0), "two"),
        group("g", Rect::new(576.0, -24.0, 248.0, 248.0)),
        inside(
            "g",
            sticky("c", Rect::new(600.0, 0.0, 200.0, 200.0), "three"),
        ),
    ]));
    app.viewport((1000.0, 800.0));
    app
}

#[track_caller]
fn answer(plan: Plan) -> Response {
    match plan {
        Plan::Answer(response) => response,
        other => panic!("expected an answer, got {other:?}"),
    }
}

#[track_caller]
fn run(plan: Plan) -> ApiRun {
    match plan {
        Plan::Run {
            event: Event::Api(ApiCall { ticket, run, .. }),
            pending,
        } => {
            assert_eq!(
                pending.ticket(),
                ticket,
                "the answer waits on the event's ticket"
            );
            run
        }
        other => panic!("expected an event, got {other:?}"),
    }
}

#[track_caller]
fn refused(plan: Plan, status: u16, error: &str) {
    let response = answer(plan);
    assert_eq!(
        (response.status, &response.body["error"]),
        (status, &json!(error))
    );
}

#[test]
fn the_canvas_is_read_as_a_json_canvas_document_with_the_one_tab() {
    let app = notes();
    let canvas = answer(api().plan(app.app(), &Request::get("/canvas"))).body;
    let ids: Vec<&Value> = canvas["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|n| &n["id"])
        .collect();
    assert_eq!(ids, ["a", "b", "g", "c"]);
    assert_eq!(canvas["edges"], json!([]));
    assert_eq!(
        canvas["specular"]["entityOrder"],
        json!(["a", "b", "g", "c"])
    );
    assert_eq!(
        canvas["appState"],
        json!({
            "zoom": 1, "pan": { "x": 0, "y": 0 },
            "activeTab": { "id": "tab_1", "name": "Canvas 1" },
            "tabs": [{ "id": "tab_1", "name": "Canvas 1", "entityCount": 4 }],
        })
    );
}

#[test]
fn a_bad_patch_is_answered_without_an_event() {
    let app = notes();
    let mut api = api();
    let rows = [
        (json!([]), "patch: expected an object"),
        (json!({ "entities": {} }), "entities: expected an array"),
        (
            json!({ "entities": ["a"] }),
            "entities[0]: expected an object",
        ),
        (
            json!({ "entities": [{ "id": "a", "text": "x" }, { "id": "missing", "text": "x" }] }),
            "entities[1]: missing or unknown kind",
        ),
        (
            json!({ "entities": [{ "kind": "shape", "fillStyle": "plaid" }] }),
            "entities[0].fillStyle: invalid value",
        ),
        (
            json!({ "entities": [{ "kind": "file" }] }),
            "entities[0]: could not be read as a file",
        ),
        (
            json!({ "entities": [{ "kind": "group", "entityIds": ["a", "nope"] }] }),
            "entities[0]: unknown entity 'nope'",
        ),
        (
            json!({ "edges": [{ "fromEntityId": "a" }] }),
            "edges[0]: fromEntityId and toEntityId are required",
        ),
        (
            json!({ "edges": [{ "fromEntityId": "a", "toEntityId": "ghost" }] }),
            "edges[0].toEntityId: unknown entity 'ghost'",
        ),
        (json!({ "delete": [1] }), "delete: expected an array of ids"),
    ];
    for (patch, error) in rows {
        let row = patch.to_string();
        let response = answer(api.plan(app.app(), &Request::post("/canvas/apply", patch)));
        assert_eq!(
            (response.status, &response.body["error"]),
            (400, &json!(error)),
            "{row}"
        );
    }
}

#[test]
fn a_page_needs_a_full_url() {
    let app = notes();
    let mut api = api();
    let mut page = |url: Value| {
        let patch = json!({ "entities": [{ "kind": "page", "url": url }] });
        api.plan(app.app(), &Request::post("/canvas/apply", patch))
    };
    for bare in [
        json!("/garden"),
        json!("localhost:4321/garden"),
        json!("http://"),
        Value::Null,
    ] {
        let response = answer(page(bare.clone()));
        let error = response.body["error"].as_str().unwrap_or_default();
        assert_eq!(response.status, 400, "{bare}");
        assert!(
            error.starts_with("entities[0]: url must be a full URL"),
            "{error}"
        );
        assert!(error.ends_with(&format!("(got {bare})")), "{error}");
    }
    for full in [
        "http://localhost:4321/garden",
        "https://example.com",
        "file:///tmp/a.html",
    ] {
        run(page(json!(full)));
    }
}

#[test]
fn an_act_route_checks_what_it_is_given() {
    let app = notes();
    let mut api = api();
    let mut post = |path: &str, body: Value| api.plan(app.app(), &Request::post(path, body));

    let rows = [
        (
            "/stack-order/bring-forward",
            json!({}),
            400,
            "id or ids is required",
        ),
        ("/groups/ungroup", json!({}), 400, "groupId is required"),
        (
            "/groups/ungroup",
            json!({ "groupId": "a" }),
            404,
            "Group not found",
        ),
        (
            "/selection/select-page",
            json!({}),
            400,
            "pageId is required",
        ),
        (
            "/groups/create",
            json!({ "label": "x" }),
            400,
            "entityIds is required",
        ),
    ];
    for (path, body, status, error) in rows {
        let response = answer(post(path, body.clone()));
        assert_eq!(
            (response.status, &response.body["error"]),
            (status, &json!(error)),
            "{path} {body}"
        );
    }
    let unknown = answer(post(
        "/stack-order/bring-forward",
        json!({ "ids": ["a", "x"] }),
    ));
    assert_eq!(unknown.status, 404);
    assert_eq!(
        unknown.body,
        json!({ "error": "Unknown stack-order id", "unknownIds": ["x"] })
    );
    assert_eq!(
        answer(post("/camera/focus", json!({ "pageIds": ["x"] }))).body,
        json!({ "focused": false })
    );
    // Nothing to undo is an answer, not an event.
    assert_eq!(
        answer(post("/history/undo", json!({}))).body,
        json!({ "ok": false, "canUndo": false, "canRedo": false })
    );
}

#[test]
fn what_is_not_ported_says_so_by_name() {
    let app = notes();
    let mut api = api();
    let mut plan = |method, path: &str| api.plan(app.app(), &Request::new(method, path, json!({})));

    let pdf = answer(plan(Method::Post, "/pages/p1/print-pdf"));
    assert_eq!(pdf.status, 501);
    let error = pdf.body["error"].as_str().unwrap_or_default();
    let start = "not implemented in the native app (it needs an answer that waits for the page";
    assert!(error.starts_with(start), "{error}");
    assert!(
        error.ends_with("): `print-pdf`. Route: POST /pages/p1/print-pdf"),
        "{error}"
    );

    refused(
        plan(Method::Post, "/tasks/component-states"),
        501,
        "not implemented in the native app (not ported yet): `component-states`. \
         Route: POST /tasks/component-states",
    );
    refused(
        plan(Method::Post, "/nope"),
        404,
        "Unknown route: POST /nope",
    );
    // The CLI's presence and heartbeat calls are taken and dropped.
    assert_eq!(
        answer(plan(Method::Post, "/session/presence")).body,
        json!({ "ok": true })
    );
    assert_eq!(
        answer(plan(Method::Post, "/mcp/session/ping")).body,
        json!({ "ok": true })
    );
    assert_eq!(
        answer(plan(Method::Get, "/health")).body,
        json!({ "version": "1" })
    );
}

#[test]
fn a_tab_ref_must_name_the_open_canvas_on_a_route_that_takes_one() {
    let app = notes();
    let mut api = api();
    let mut tabbed = |path: &str, tab: &str| {
        let request = Request {
            tab: Some(tab.to_owned()),
            ..Request::get(path)
        };
        api.plan(app.app(), &request)
    };
    assert_eq!(answer(tabbed("/canvas", "Canvas 1")).status, 200);
    assert_eq!(answer(tabbed("/canvas", "tab_1")).status, 200);
    assert_eq!(
        answer(tabbed("/selection", "")).status,
        200,
        "an empty ref is no ref"
    );
    refused(
        tabbed("/canvas", "other"),
        400,
        "unknown tab 'other' \u{2014} available: tab_1 (Canvas 1)",
    );
    refused(
        tabbed("/selection", "Canvas 1"),
        400,
        "--tab is not supported for GET /selection",
    );
}
