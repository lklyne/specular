//! `Api::plan` on its own: a request and an app in, an answer or an event
//! out, and nothing changed.
#![expect(
    clippy::panic,
    reason = "a helper panics to fail the test it is called from"
)]

use serde_json::{Value, json};
use specular_api::{Api, Method, Plan, Request, Response, Tab};
use specular_doc::{Command, ItemId, Rect};
use specular_interact::{Action, ApiCall, ApiRun, Event};
use specular_testkit::{TestApp, document, group, inside, sticky};

fn api() -> Api {
    let tab = Tab {
        id: "tab_1".to_owned(),
        name: "Canvas".to_owned(),
    };
    Api::new(tab, 0)
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
            event: Event::Api(ApiCall { ticket, run }),
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

fn entity(id: &str) -> ItemId {
    ItemId::Entity(id.into())
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
            "activeTab": { "id": "tab_1", "name": "Canvas" },
            "tabs": [{ "id": "tab_1", "name": "Canvas", "entityCount": 4 }],
        })
    );
}

#[test]
fn a_patch_becomes_one_batch_for_update() {
    let app = notes();
    let patch = json!({
        "entities": [{ "kind": "text", "text": "new", "canvasX": 40, "canvasY": 400 }, { "id": "a", "text": "ONE" }],
        "edges": [{ "fromEntityId": "a", "toEntityId": "b" }],
        "delete": ["b"],
    });
    let ApiRun::Apply {
        command: Command::Batch(commands),
        select: None,
    } = run(api().plan(app.app(), &Request::post("/canvas/apply", patch)))
    else {
        panic!("a patch is one batch");
    };
    let names: Vec<&str> = (commands.iter())
        .map(|command| match command {
            Command::InsertEntity { .. } => "insert entity",
            Command::SetKind { .. } => "set kind",
            Command::InsertEdge { .. } => "insert edge",
            Command::RemoveEdge(_) => "remove edge",
            Command::RemoveEntity(_) => "remove entity",
            _ => "other",
        })
        .collect();
    // Deleting `b` takes the edge the same patch gave it.
    assert_eq!(
        names,
        [
            "insert entity",
            "set kind",
            "insert edge",
            "remove edge",
            "remove entity"
        ]
    );
    // Planning read the app and left it alone.
    assert_eq!(app.entity("a").label, None);
    assert!(!app.app().can_undo());
}

#[test]
fn a_bad_patch_is_answered_without_an_event() {
    let app = notes();
    let mut api = api();
    let mut apply = |patch: Value| api.plan(app.app(), &Request::post("/canvas/apply", patch));
    refused(apply(json!([])), 400, "patch: expected an object");
    refused(
        apply(json!({ "entities": {} })),
        400,
        "entities: expected an array",
    );
    refused(
        apply(json!({ "entities": ["a"] })),
        400,
        "entities[0]: expected an object",
    );
    refused(
        apply(
            json!({ "entities": [{ "id": "a", "text": "x" }, { "id": "missing", "text": "x" }] }),
        ),
        400,
        "entities[1]: missing or unknown kind",
    );
    refused(
        apply(json!({ "entities": [{ "kind": "shape", "fillStyle": "plaid" }] })),
        400,
        "entities[0].fillStyle: invalid value",
    );
    refused(
        apply(json!({ "entities": [{ "kind": "file" }] })),
        400,
        "entities[0]: could not be read as a file",
    );
    refused(
        apply(json!({ "entities": [{ "kind": "group", "entityIds": ["a", "nope"] }] })),
        400,
        "entities[0]: unknown entity 'nope'",
    );
    refused(
        apply(json!({ "edges": [{ "fromEntityId": "a" }] })),
        400,
        "edges[0]: fromEntityId and toEntityId are required",
    );
    refused(
        apply(json!({ "edges": [{ "fromEntityId": "a", "toEntityId": "ghost" }] })),
        400,
        "edges[0].toEntityId: unknown entity 'ghost'",
    );
    refused(
        apply(json!({ "delete": [1] })),
        400,
        "delete: expected an array of ids",
    );
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
fn the_act_routes_map_onto_the_window_s_actions() {
    let app = notes();
    let mut api = api();
    let mut post = |path: &str, body: Value| api.plan(app.app(), &Request::post(path, body));

    assert_eq!(
        run(post(
            "/stack-order/bring-to-front",
            json!({ "ids": ["a", "b"] })
        )),
        ApiRun::Act {
            on: Some(vec![entity("a"), entity("b")]),
            action: Action::BringToFront
        }
    );
    assert_eq!(
        run(post("/stack-order/send-backward", json!({ "id": "c" }))),
        ApiRun::Act {
            on: Some(vec![entity("c")]),
            action: Action::SendBackward
        }
    );
    assert_eq!(
        run(post("/groups/ungroup", json!({ "groupId": "g" }))),
        ApiRun::Act {
            on: Some(vec![entity("g")]),
            action: Action::Ungroup
        }
    );
    assert_eq!(
        run(post(
            "/selection/select-entities",
            json!({ "entityIds": ["a", "gone", "c"] })
        )),
        ApiRun::Act {
            on: None,
            action: Action::Select(vec![entity("a"), entity("c")])
        }
    );
    assert_eq!(
        run(post("/selection/deselect", json!({}))),
        ApiRun::Act {
            on: None,
            action: Action::Select(Vec::new())
        }
    );
    // `a` is 200 square at the origin: centred in 1000x800 at 100%.
    let ApiRun::Act {
        on,
        action: Action::SetCamera(camera),
    } = run(post("/camera/focus", json!({ "pageIds": ["a"] })))
    else {
        panic!("focus moves the camera");
    };
    assert_eq!(on, Some(vec![entity("a")]));
    assert_eq!(
        (camera.zoom, camera.pan.x, camera.pan.y),
        (1.0, 400.0, 300.0)
    );
}

#[test]
fn an_act_route_checks_what_it_is_given() {
    let app = notes();
    let mut api = api();
    let mut post = |path: &str, body: Value| api.plan(app.app(), &Request::post(path, body));

    refused(
        post("/stack-order/bring-forward", json!({})),
        400,
        "id or ids is required",
    );
    let unknown = answer(post(
        "/stack-order/bring-forward",
        json!({ "ids": ["a", "x"] }),
    ));
    assert_eq!(unknown.status, 404);
    assert_eq!(
        unknown.body,
        json!({ "error": "Unknown stack-order id", "unknownIds": ["x"] })
    );
    refused(
        post("/groups/ungroup", json!({})),
        400,
        "groupId is required",
    );
    refused(
        post("/groups/ungroup", json!({ "groupId": "a" })),
        404,
        "Group not found",
    );
    refused(
        post("/selection/select-page", json!({})),
        400,
        "pageId is required",
    );
    refused(
        post("/groups/create", json!({ "label": "x" })),
        400,
        "entityIds is required",
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

    let snapshot = answer(plan(Method::Post, "/pages/p1/snapshot-seen"));
    assert_eq!(snapshot.status, 501);
    let error = snapshot.body["error"].as_str().unwrap_or_default();
    let start = "not implemented in the native app (needs the CEF page backend): \
                 the page verbs (`snapshot`, `screenshot -f`, `click`,";
    assert!(error.starts_with(start), "{error}");
    assert!(
        error.ends_with(". Route: POST /pages/p1/snapshot-seen"),
        "{error}"
    );

    refused(
        plan(Method::Post, "/selection/arrange"),
        501,
        "not implemented in the native app (not ported yet): `arrange`. \
         Route: POST /selection/arrange",
    );
    refused(
        plan(Method::Post, "/tabs/switch"),
        501,
        "not implemented in the native app (this app has one canvas open): \
         `tab new`, `tab switch` and `tab delete`. Route: POST /tabs/switch",
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
    assert_eq!(answer(tabbed("/canvas", "Canvas")).status, 200);
    assert_eq!(answer(tabbed("/canvas", "tab_1")).status, 200);
    refused(
        tabbed("/canvas", "other"),
        400,
        "no tab matches 'other'. Open tabs: Canvas (tab_1)",
    );
    refused(
        tabbed("/selection", "Canvas"),
        400,
        "--tab is not supported for GET /selection",
    );
}
