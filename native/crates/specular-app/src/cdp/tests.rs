//! The endpoints over real sockets with the synthetic page source: the
//! test's own thread plays the event loop.

use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use serde_json::{Value, json};
use specular_api::cdp::Target;
use specular_core::{CssSize, PageId, PageSource, PageSpec, SyntheticPageSource};
use specular_doc::EntityId;

use super::CdpHost;
use super::test_client::WsClient;

/// The endpoints, the pages behind them, and the wakes the sockets send.
struct Rig {
    host: CdpHost,
    source: SyntheticPageSource,
    wakes: Receiver<()>,
    pages: Vec<(EntityId, PageId)>,
}

impl Rig {
    /// `count` pages, `p1` up, each at `https://example.com/pN`.
    fn new(count: usize) -> Self {
        let (wake, wakes) = mpsc::channel();
        let host = CdpHost::start(move || {
            let _ = wake.send(());
        })
        .unwrap();
        let mut source = SyntheticPageSource::new();
        source.set_devtools_sink(Some(host.sink()));
        let pages = (1..=count)
            .map(|n| {
                let spec =
                    PageSpec::new(&format!("https://example.com/p{n}"), CssSize::new(400, 300));
                (
                    EntityId::from(format!("p{n}").as_str()),
                    source.create_page(&spec).unwrap(),
                )
            })
            .collect();
        Self {
            host,
            source,
            wakes,
            pages,
        }
    }

    /// The websocket path `GET /pages/<id>/cdp-target` answers with.
    fn path(&self, index: usize) -> String {
        let (entity, page) = &self.pages[index];
        let target = Target {
            id: entity.as_str().to_owned(),
            url: format!("https://example.com/{entity}"),
            title: String::new(),
        };
        let body = self.host.target(entity, *page, &target);
        let url = body["webSocketDebuggerUrl"].as_str().unwrap();
        url.split_once(&self.port().to_string())
            .unwrap()
            .1
            .to_owned()
    }

    fn port(&self) -> u16 {
        self.host.server.port()
    }

    fn connect(&self, index: usize) -> WsClient {
        WsClient::connect(self.port(), &self.path(index)).unwrap()
    }

    /// One turn of the event loop, once a client's message has arrived.
    fn turn(&mut self) {
        self.wakes.recv_timeout(Duration::from_secs(5)).unwrap();
        self.host.serve(&mut self.source);
        self.source.pump();
    }
}

fn recv(client: &mut WsClient) -> Value {
    serde_json::from_str(&client.recv().unwrap()).unwrap()
}

/// Attaches as agent-browser does and returns the session.
fn attach(client: &mut WsClient, target: &str) -> String {
    let ask = json!({ "id": 1, "method": "Target.attachToTarget",
        "params": { "targetId": target, "flatten": true } });
    client.send(&ask.to_string()).unwrap();
    assert_eq!(recv(client)["method"], "Target.attachedToTarget");
    recv(client)["result"]["sessionId"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn evaluate(rig: &mut Rig, client: &mut WsClient, session: &str, expression: &str) -> Value {
    let ask = json!({ "id": 2, "sessionId": session, "method": "Runtime.evaluate",
        "params": { "expression": expression } });
    client.send(&ask.to_string()).unwrap();
    rig.turn();
    let reply = recv(client);
    assert_eq!(reply["id"], 2);
    assert_eq!(reply["sessionId"], session);
    reply["result"]["result"]["value"].clone()
}

#[test]
fn each_socket_speaks_for_its_own_page_with_twenty_open() {
    let mut rig = Rig::new(20);
    let mut clients: Vec<(usize, WsClient, String)> = [0, 7, 19]
        .into_iter()
        .map(|index| {
            let mut client = rig.connect(index);
            let session = attach(&mut client, &format!("p{}", index + 1));
            (index, client, session)
        })
        .collect();
    // Asked last to first, so an answer cannot land by arrival order.
    for (index, client, session) in clients.iter_mut().rev() {
        let href = evaluate(&mut rig, client, session, "location.href");
        assert_eq!(href, format!("https://example.com/p{}", *index + 1));
    }
    // Each client is shown one target: its own.
    let (_, client, _) = &mut clients[1];
    client
        .send(r#"{"id":9,"method":"Target.getTargets"}"#)
        .unwrap();
    let targets = recv(client)["result"]["targetInfos"].clone();
    assert_eq!(targets.as_array().unwrap().len(), 1);
    assert_eq!(targets[0]["targetId"], "p8");
}

#[test]
fn a_page_keeps_one_address_and_an_unknown_token_is_refused() {
    let rig = Rig::new(2);
    assert_eq!(rig.path(0), rig.path(0));
    assert_ne!(rig.path(0), rig.path(1));
    assert!(WsClient::connect(rig.port(), "/cdp/page/not-a-token").is_err());
    assert!(WsClient::connect(rig.port(), "/devtools/browser").is_err());
}

#[test]
fn a_navigation_through_the_socket_moves_only_that_page() {
    let mut rig = Rig::new(2);
    let mut first = rig.connect(0);
    let mut second = rig.connect(1);
    let (one, two) = (attach(&mut first, "p1"), attach(&mut second, "p2"));
    let go = json!({ "id": 3, "sessionId": one, "method": "Page.navigate",
        "params": { "url": "https://example.org/next" } });
    first.send(&go.to_string()).unwrap();
    rig.turn();
    assert_eq!(recv(&mut first)["id"], 3);
    assert_eq!(
        evaluate(&mut rig, &mut first, &one, "location.href"),
        "https://example.org/next"
    );
    assert_eq!(
        evaluate(&mut rig, &mut second, &two, "location.href"),
        "https://example.com/p2"
    );
}

#[test]
fn a_closed_page_hangs_up_on_its_clients_and_its_address_stops_working() {
    let mut rig = Rig::new(2);
    let path = rig.path(0);
    let mut first = rig.connect(0);
    let mut second = rig.connect(1);
    let two = attach(&mut second, "p2");
    let entity = rig.pages[0].0.clone();
    rig.host.page_closed(&entity);
    assert_eq!(
        first.recv().unwrap_err().kind(),
        std::io::ErrorKind::ConnectionAborted
    );
    assert!(WsClient::connect(rig.port(), &path).is_err());
    assert_eq!(
        evaluate(&mut rig, &mut second, &two, "document.readyState"),
        "complete"
    );
}

#[test]
fn a_message_the_page_would_not_take_is_answered_with_the_reason() {
    let mut rig = Rig::new(1);
    let mut client = rig.connect(0);
    let session = attach(&mut client, "p1");
    // The backend lost the page before the endpoint heard of it.
    let page = rig.pages[0].1;
    rig.source.close_page(page).unwrap();
    let ask = json!({ "id": 4, "sessionId": session, "method": "Page.enable" });
    client.send(&ask.to_string()).unwrap();
    rig.turn();
    let reply = recv(&mut client);
    assert_eq!(reply["id"], 4);
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unknown page")
    );
}

#[test]
fn generations_count_navigations_and_remember_the_last_snapshot() {
    let rig = Rig::new(1);
    let (entity, page) = rig.pages[0].clone();
    let target = Target {
        id: "p1".to_owned(),
        url: String::new(),
        title: String::new(),
    };
    let read = |rig: &Rig| {
        let body = rig.host.target(&entity, page, &target);
        (
            body["generation"].clone(),
            body["lastSnapshotGeneration"].clone(),
        )
    };
    assert_eq!(read(&rig), (json!(0), Value::Null));
    rig.host.navigated(&entity, "https://example.com/a");
    assert_eq!(rig.host.snapshot_seen(&entity), 1);
    rig.host.navigated(&entity, "https://example.com/b");
    assert_eq!(read(&rig), (json!(2), json!(1)));
}
