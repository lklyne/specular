//! The page bridge's messages: a page's `{id, op, nonce, ..}` planned as
//! the API's own routes, its `run` an event through `update` and one undo
//! step, and pushes for the keys a page subscribed to.

use serde_json::{Value, json};
use specular_api::Api;
use specular_api::bridge::{BridgeStep, Subscriptions};
use specular_doc::Rect;
use specular_interact::{
    Action, ApiCall, ApiRun, Effect, Event, NewItem, NewKind, Tool, state_key,
};
use specular_testkit::{TestApp, document, sticky};

const NONCE: &str = "n0nce";

fn one_note() -> TestApp {
    let mut app = TestApp::from_document(document([sticky(
        "a",
        Rect::new(0.0, 0.0, 200.0, 200.0),
        "one",
    )]));
    app.viewport((1000.0, 800.0));
    app
}

fn call(id: u64, op: &str, fields: Value) -> String {
    let mut call = json!({ "id": id, "op": op, "nonce": NONCE });
    if let (Some(call), Value::Object(fields)) = (call.as_object_mut(), fields) {
        call.extend(fields);
    }
    call.to_string()
}

/// The message an answer or a push script hands the page.
fn message(script: &str) -> Value {
    let start = script.find("__specularReceive(").map_or(0, |at| at + 18);
    let inner = script[start..].strip_suffix(')').unwrap_or_default();
    serde_json::from_str(inner).unwrap_or(Value::Null)
}

/// The message a reply carries; any other step is `null`, which no
/// expected answer is.
fn reply(step: BridgeStep) -> Value {
    match step {
        BridgeStep::Reply(script) => message(&script),
        _ => Value::Null,
    }
}

#[test]
fn a_pages_run_is_an_api_event_through_update_and_one_undo_step() {
    let mut app = one_note();
    let mut api = Api::new(1);
    let args = json!({ "kind": "sticky", "x": 400, "y": 100, "text": "hi" });
    let payload = call(7, "run", json!({ "name": "item.add", "args": args }));
    let BridgeStep::Run { event, pending } = api.bridge(app.app(), &payload, NONCE) else {
        panic!("a run plans an event");
    };
    let Event::Api(ApiCall {
        run: ApiRun::Act { on: None, action },
        ..
    }) = &event
    else {
        panic!("a run is an API act, as POST /run sends: {event:?}");
    };
    let placed = NewItem {
        kind: NewKind::Sticky,
        at: Some(glam::DVec2::new(400.0, 100.0)),
        text: "hi".to_owned(),
    };
    assert_eq!(*action, Action::AddItem(placed));
    app.take_effects();
    app.send(event);
    let outcome = (app.take_effects().into_iter()).find_map(|effect| match effect {
        Effect::ApiReply { outcome, .. } => Some(outcome),
        _ => None,
    });
    let answer = message(&pending.finish(app.app(), outcome.as_ref()));
    assert_eq!(
        answer,
        json!({ "id": 7, "ok": true, "value": { "ok": true, "name": "item.add" } })
    );
    assert_eq!(app.document().entities().count(), 2);
    app.assert_undo_returns_to_start();
}

#[test]
fn reads_answer_from_the_state_and_the_registry_and_refusals_answer_with_why() {
    let app = one_note();
    let mut api = Api::new(1);
    let toolbar = state_key(app.app(), "panels/toolbar").unwrap_or_default();
    let state = reply(api.bridge(
        app.app(),
        &call(1, "state", json!({ "key": "panels/toolbar" })),
        NONCE,
    ));
    assert_eq!(state, json!({ "id": 1, "ok": true, "value": toolbar }));
    let commands = reply(api.bridge(app.app(), &call(2, "commands", json!({})), NONCE));
    let names: Vec<&str> = (commands["value"].as_array().into_iter().flatten())
        .filter_map(|command| command["name"].as_str())
        .collect();
    assert!(names.contains(&"item.add"), "{names:?}");
    let refused = [
        (
            call(3, "run", json!({ "name": "item.ad" })),
            "did you mean item.add",
        ),
        (
            call(4, "run", json!({ "name": "edit.redo" })),
            "cannot run now",
        ),
        (
            call(5, "run", json!({ "name": "app.quit" })),
            "the window's own",
        ),
        (
            call(6, "state", json!({ "key": "camera" })),
            "no state key `camera`",
        ),
        (
            call(7, "subscribe", json!({ "key": "camera" })),
            "no state key",
        ),
        (
            call(8, "eval", json!({ "code": "1" })),
            "no bridge op `eval`",
        ),
    ];
    for (payload, why) in refused {
        let answer = reply(api.bridge(app.app(), &payload, NONCE));
        assert_eq!(answer["ok"], false, "{payload}: {answer}");
        let error = answer["error"].as_str().unwrap_or_default();
        assert!(error.contains(why), "{payload}: {error:?} lacks {why:?}");
    }
    let unanswered = [
        call(9, "commands", json!({ "nonce": "someone else" })),
        "not json".to_owned(),
        json!({ "op": "commands", "nonce": NONCE }).to_string(),
    ];
    for payload in unanswered {
        let step = api.bridge(app.app(), &payload, NONCE);
        assert_eq!(step, BridgeStep::Ignore, "{payload}");
    }
}

#[test]
fn a_subscribed_key_is_pushed_when_it_moves_and_only_to_its_subscribers() {
    let mut app = one_note();
    let mut api = Api::new(1);
    let mut subscriptions = Subscriptions::default();
    let payload = call(1, "subscribe", json!({ "key": "panels/toolbar" }));
    let BridgeStep::Subscribe { key, value, reply } = api.bridge(app.app(), &payload, NONCE) else {
        panic!("a subscribe step");
    };
    assert_eq!(message(&reply)["value"], value);
    subscriptions.subscribe("widget", key, value);
    subscriptions.subscribe(
        "other",
        "theme",
        state_key(app.app(), "theme").unwrap_or_default(),
    );
    assert!(
        subscriptions.changes(app.app()).is_empty(),
        "nothing moved yet"
    );
    app.send(Event::Action(Action::SetTool(Tool::Draw)));
    let pushed = subscriptions.changes(app.app());
    let toolbar = state_key(app.app(), "panels/toolbar").unwrap_or_default();
    let pages: Vec<&str> = pushed.iter().map(|(page, _)| *page).collect();
    assert_eq!(pages, ["widget"], "the toolbar moved, the theme did not");
    assert_eq!(
        message(&pushed[0].1),
        json!({ "key": "panels/toolbar", "value": toolbar })
    );
    assert!(subscriptions.changes(app.app()).is_empty(), "pushed once");
    subscriptions.unsubscribe(&"widget", "panels/toolbar");
    app.send(Event::Action(Action::SetTool(Tool::Select)));
    assert!(subscriptions.changes(app.app()).is_empty(), "unsubscribed");
}

#[test]
fn a_page_follows_a_thread_by_its_key_before_and_after_it_is_made() {
    let mut app = one_note();
    let mut api = Api::new(1);
    let mut subscriptions = Subscriptions::default();
    let payload = call(1, "subscribe", json!({ "key": "chat/thread/t1" }));
    let BridgeStep::Subscribe { key, value, .. } = api.bridge(app.app(), &payload, NONCE) else {
        panic!("a thread's key is a key, made or not");
    };
    assert_eq!(value, Value::Null, "no thread t1 yet");
    subscriptions.subscribe("block", key, value);
    let start = specular_interact::ChatAction::NewThread {
        id: Some(specular_interact::ThreadId("t1".to_owned())),
        open: false,
    };
    app.act(Action::Chat(start));
    let pushed = subscriptions.changes(app.app());
    assert_eq!(pushed.len(), 1, "{pushed:?}");
    assert_eq!(message(&pushed[0].1)["value"]["id"], "t1");
    let bare = call(2, "subscribe", json!({ "key": "chat/thread/" }));
    let refused = reply(api.bridge(app.app(), &bare, NONCE));
    assert_eq!(refused["ok"], false, "{refused}");
}
