//! The page proxy's routing, against a fake devtools channel: a page that
//! keeps what it was sent and answers by id.

use serde_json::{Value, json};
use specular_api::cdp::{ClientId, Out, PageProxy, Target};
use specular_core::DEVTOOLS_CLIENT_ID_BASE;

/// A page's devtools channel and the clients' sockets, in memory.
struct Wire {
    proxy: PageProxy,
    /// What the page was sent, in order.
    to_page: Vec<Value>,
    /// What each client was sent, in order.
    to_client: Vec<(ClientId, Value)>,
}

fn parse(text: &str) -> Value {
    serde_json::from_str(text).unwrap_or(Value::Null)
}

impl Wire {
    fn new(page: &str) -> Self {
        let target = Target {
            id: page.to_owned(),
            url: format!("https://example.com/{page}"),
            title: page.to_uppercase(),
        };
        Self {
            proxy: PageProxy::new(target),
            to_page: Vec::new(),
            to_client: Vec::new(),
        }
    }

    fn deliver(&mut self, out: Vec<Out>) {
        for out in out {
            match out {
                Out::ToClient(client, text) => self.to_client.push((client, parse(&text))),
                Out::ToPage { id, message } => {
                    let message = parse(&message);
                    assert_eq!(message["id"], id, "the id on the wire is the one reported");
                    self.to_page.push(message);
                }
            }
        }
    }

    fn client_says(&mut self, client: ClientId, message: &Value) {
        let out = self.proxy.from_client(client, &message.to_string());
        self.deliver(out);
    }

    fn page_says(&mut self, message: &Value) {
        let out = self.proxy.from_page(&message.to_string());
        self.deliver(out);
    }

    /// Everything `client` has been sent since the last call.
    fn heard(&mut self, client: ClientId) -> Vec<Value> {
        let (mine, rest) = (self.to_client.drain(..)).partition(|(to, _)| *to == client);
        self.to_client = rest;
        mine.into_iter().map(|(_, message)| message).collect()
    }

    /// Connects `client` and attaches it, as agent-browser does on
    /// connecting. Returns its session id.
    fn attach(&mut self, client: ClientId) -> String {
        self.proxy.connect(client);
        let target = self.proxy.target().id.clone();
        let ask = json!({ "id": 1, "method": "Target.attachToTarget",
            "params": { "targetId": target, "flatten": true } });
        self.client_says(client, &ask);
        let heard = self.heard(client);
        (heard[1]["result"]["sessionId"].as_str())
            .unwrap_or_default()
            .to_owned()
    }
}

#[test]
fn a_client_sees_one_target_and_the_page_is_not_asked() {
    let mut wire = Wire::new("p2");
    wire.proxy.connect(7);
    wire.client_says(7, &json!({ "id": 1, "method": "Target.getTargets" }));
    wire.client_says(
        7,
        &json!({ "id": 2, "method": "Target.setDiscoverTargets", "params": { "discover": true } }),
    );
    wire.client_says(7, &json!({ "id": 3, "method": "Browser.getVersion" }));
    let heard = wire.heard(7);
    let info = json!({
        "targetId": "p2", "type": "page", "title": "P2", "url": "https://example.com/p2",
        "attached": false, "canAccessOpener": false, "browserContextId": "specular",
    });
    assert_eq!(
        heard[0],
        json!({ "id": 1, "result": { "targetInfos": [info] } })
    );
    assert_eq!(heard[1]["method"], "Target.targetCreated");
    assert_eq!(heard[1]["params"]["targetInfo"], info);
    assert_eq!(heard[2], json!({ "id": 2, "result": {} }));
    assert_eq!(heard[3]["result"]["protocolVersion"], "1.3");
    assert_eq!(wire.to_page, [] as [Value; 0]);
}

#[test]
fn attaching_announces_a_session_and_only_this_page_can_be_attached() {
    let mut wire = Wire::new("p2");
    wire.proxy.connect(7);
    let attach = |target: &str| {
        json!({ "id": 5, "method": "Target.attachToTarget",
            "params": { "targetId": target, "flatten": true } })
    };
    wire.client_says(7, &attach("p1"));
    let refused = wire.heard(7);
    assert_eq!(
        refused[0]["error"]["message"],
        "No target with given id found"
    );

    wire.client_says(7, &attach("p2"));
    let heard = wire.heard(7);
    assert_eq!(heard[0]["method"], "Target.attachedToTarget");
    assert_eq!(heard[0]["params"]["targetInfo"]["attached"], true);
    assert_eq!(heard[0]["params"]["waitingForDebugger"], false);
    let session = heard[0]["params"]["sessionId"].clone();
    assert_eq!(
        heard[1],
        json!({ "id": 5, "result": { "sessionId": session } })
    );
}

#[test]
fn a_session_message_reaches_the_page_bare_and_its_answer_comes_back_dressed() {
    let mut wire = Wire::new("p1");
    let session = wire.attach(7);
    let params = json!({ "expression": "document.title" });
    wire.client_says(
        7,
        &json!({ "id": 12, "sessionId": session, "method": "Runtime.evaluate", "params": params }),
    );
    let sent = wire.to_page.pop().unwrap();
    let upstream = sent["id"].as_i64().unwrap();
    assert!(upstream >= i64::from(DEVTOOLS_CLIENT_ID_BASE));
    assert_eq!(
        sent,
        json!({ "id": upstream, "method": "Runtime.evaluate", "params": params })
    );

    let result = json!({ "result": { "type": "string", "value": "P1" } });
    wire.page_says(&json!({ "id": upstream, "result": result }));
    assert_eq!(
        wire.heard(7),
        [json!({ "id": 12, "sessionId": session, "result": result })]
    );
    // The page answers once; a second answer to the same id goes nowhere.
    wire.page_says(&json!({ "id": upstream, "result": {} }));
    assert_eq!(wire.heard(7), [] as [Value; 0]);
}

#[test]
fn two_clients_using_the_same_ids_each_get_their_own_answer() {
    let mut wire = Wire::new("p1");
    let (first, second) = (wire.attach(1), wire.attach(2));
    for (client, session) in [(1, &first), (2, &second)] {
        wire.client_says(
            client,
            &json!({ "id": 3, "sessionId": session, "method": "Page.enable" }),
        );
    }
    let ids: Vec<i64> = (wire.to_page.iter())
        .map(|sent| sent["id"].as_i64().unwrap())
        .collect();
    assert_ne!(ids[0], ids[1]);
    // Answered out of order.
    wire.page_says(&json!({ "id": ids[1], "result": { "for": "second" } }));
    wire.page_says(&json!({ "id": ids[0], "result": { "for": "first" } }));
    assert_eq!(wire.heard(1)[0]["result"]["for"], "first");
    assert_eq!(wire.heard(2)[0]["result"]["for"], "second");
}

#[test]
fn a_page_event_goes_to_every_session_and_to_no_client_without_one() {
    let mut wire = Wire::new("p1");
    let (first, second) = (wire.attach(1), wire.attach(2));
    wire.proxy.connect(3);
    let event = json!({ "method": "Page.loadEventFired", "params": { "timestamp": 1 } });
    wire.page_says(&event);
    for (client, session) in [(1, first), (2, second)] {
        let mut dressed = event.clone();
        dressed["sessionId"] = json!(session);
        assert_eq!(wire.heard(client), [dressed]);
    }
    assert_eq!(wire.heard(3), [] as [Value; 0]);
}

#[test]
fn a_session_the_page_made_itself_is_passed_through_both_ways() {
    let mut wire = Wire::new("p1");
    wire.attach(1);
    wire.proxy.connect(2);
    // An iframe the page auto-attached: the page names the session.
    wire.client_says(
        1,
        &json!({ "id": 4, "sessionId": "FRAME", "method": "Runtime.enable" }),
    );
    let sent = wire.to_page.pop().unwrap();
    assert_eq!(sent["sessionId"], "FRAME");
    wire.page_says(&json!({ "id": sent["id"], "sessionId": "FRAME", "result": {} }));
    assert_eq!(
        wire.heard(1),
        [json!({ "id": 4, "sessionId": "FRAME", "result": {} })]
    );
    let event = json!({ "method": "Runtime.consoleAPICalled", "sessionId": "FRAME", "params": {} });
    wire.page_says(&event);
    assert_eq!(wire.heard(1), [event]);
    assert_eq!(wire.heard(2), [] as [Value; 0], "no session, no event");
}

#[test]
fn what_would_take_the_page_away_is_refused_and_never_reaches_it() {
    let mut wire = Wire::new("p1");
    let session = wire.attach(1);
    wire.client_says(
        1,
        &json!({ "id": 8, "method": "Target.closeTarget", "params": { "targetId": "p1" } }),
    );
    wire.client_says(
        1,
        &json!({ "id": 9, "sessionId": session, "method": "Page.close" }),
    );
    wire.client_says(
        1,
        &json!({ "id": 10, "method": "Target.createTarget", "params": { "url": "about:blank" } }),
    );
    let heard = wire.heard(1);
    assert_eq!(heard.len(), 3);
    for reply in &heard {
        let message = reply["error"]["message"].as_str().unwrap();
        assert!(
            message.contains("Specular owns page lifecycle"),
            "{message}"
        );
    }
    assert_eq!(heard[1]["sessionId"], json!(session));
    assert_eq!(wire.to_page, [] as [Value; 0]);
}

#[test]
fn a_client_that_left_gets_nothing_and_an_unsent_message_is_an_error() {
    let mut wire = Wire::new("p1");
    let session = wire.attach(1);
    wire.client_says(
        1,
        &json!({ "id": 2, "sessionId": session, "method": "DOM.getDocument" }),
    );
    let first = wire.to_page.pop().unwrap()["id"].as_i64().unwrap();
    // The channel would not take it: the client hears why.
    let refused = wire.proxy.refuse(first as i32, "the page is gone").unwrap();
    wire.deliver(vec![refused]);
    assert_eq!(
        wire.heard(1),
        [json!({ "id": 2, "sessionId": session,
            "error": { "code": -32000, "message": "the page is gone" } })]
    );

    wire.client_says(
        1,
        &json!({ "id": 3, "sessionId": session, "method": "DOM.getDocument" }),
    );
    let second = wire.to_page.pop().unwrap()["id"].clone();
    wire.proxy.disconnect(1);
    wire.page_says(&json!({ "id": second, "result": {} }));
    wire.page_says(&json!({ "method": "Page.loadEventFired", "params": {} }));
    assert_eq!(wire.to_client, []);
    assert!(!wire.proxy.has_clients());
}

#[test]
fn auto_attach_at_the_browser_level_attaches_the_one_page_once() {
    let mut wire = Wire::new("p1");
    wire.proxy.connect(1);
    let ask = json!({ "id": 1, "method": "Target.setAutoAttach",
        "params": { "autoAttach": true, "waitForDebuggerOnStart": true, "flatten": true } });
    wire.client_says(1, &ask);
    let heard = wire.heard(1);
    assert_eq!(heard[0]["method"], "Target.attachedToTarget");
    assert_eq!(heard[1], json!({ "id": 1, "result": {} }));
    wire.client_says(1, &ask);
    assert_eq!(wire.heard(1), [json!({ "id": 1, "result": {} })]);
    assert_eq!(wire.to_page, [] as [Value; 0]);
}

#[test]
fn browser_calls_are_answered_by_the_proxy_and_session_calls_go_to_the_page() {
    let mut wire = Wire::new("p1");
    let session = wire.attach(1);
    let call = |id: i64, method: &str, params: Value| json!({ "id": id, "method": method, "params": params });

    // A call for another target is refused, for this one it is answered.
    wire.client_says(
        1,
        &call(2, "Target.getTargetInfo", json!({ "targetId": "p9" })),
    );
    wire.client_says(
        1,
        &call(3, "Target.getTargetInfo", json!({ "targetId": "p1" })),
    );
    // An attach that names no target is refused, discovery switched off sends no event.
    wire.client_says(
        1,
        &call(4, "Target.attachToTarget", json!({ "flatten": true })),
    );
    wire.client_says(
        1,
        &call(5, "Target.setDiscoverTargets", json!({ "discover": false })),
    );
    let heard = wire.heard(1);
    assert_eq!(
        heard[0]["error"]["message"],
        "No target with given id found"
    );
    assert_eq!(heard[1]["result"]["targetInfo"]["targetId"], "p1");
    assert_eq!(
        heard[2]["error"]["message"],
        "No target with given id found"
    );
    assert_eq!(heard[3], json!({ "id": 5, "result": {} }));
    assert_eq!(heard.len(), 4);

    // A session-scoped call is the page's own, whatever its name.
    let mut scoped = call(6, "Target.getTargets", json!({}));
    scoped["sessionId"] = json!(session);
    wire.client_says(1, &scoped);
    assert_eq!(wire.to_page.len(), 1);
    assert_eq!(wire.heard(1), [] as [Value; 0]);
    wire.to_page.clear();

    // Each attach is a session of its own, and a detach names one that is held.
    wire.client_says(
        1,
        &call(7, "Target.attachToTarget", json!({ "targetId": "p1" })),
    );
    let second = wire.heard(1)[0]["params"]["sessionId"].clone();
    assert_ne!(second, json!(session));
    wire.client_says(
        1,
        &call(8, "Target.detachFromTarget", json!({ "sessionId": second })),
    );
    let detached = wire.heard(1);
    assert_eq!(detached[0]["method"], "Target.detachedFromTarget");
    assert_eq!(detached[1], json!({ "id": 8, "result": {} }));
    wire.client_says(
        1,
        &call(9, "Target.detachFromTarget", json!({ "sessionId": second })),
    );
    assert_eq!(
        wire.heard(1)[0]["error"]["message"],
        "No session with given id"
    );

    // A refused call is forgotten: a late answer to it goes nowhere.
    let mut ask = call(10, "DOM.getDocument", json!({}));
    ask["sessionId"] = json!(session);
    wire.client_says(1, &ask);
    let upstream = wire.to_page.pop().unwrap()["id"].as_i64().unwrap();
    let refused = wire
        .proxy
        .refuse(i32::try_from(upstream).unwrap(), "gone")
        .unwrap();
    wire.deliver(vec![refused]);
    wire.heard(1);
    wire.page_says(&json!({ "id": upstream, "result": {} }));
    assert_eq!(wire.heard(1), [] as [Value; 0]);
}
