//! The page routes: history verbs, the debugging target, and a changed
//! address navigating the page in place.

use serde_json::json;
use specular_core::PageNav;
use specular_interact::{Effect, PageNotice};
use specular_testkit::TestApp;

use crate::common::Scripted;

fn session() -> Scripted {
    let mut app = TestApp::with_pages(2);
    app.viewport((1000.0, 800.0));
    Scripted::new(app)
}

fn navigations(session: &Scripted) -> Vec<(String, PageNav)> {
    (session.effects.iter())
        .filter_map(|effect| match effect {
            Effect::Navigate { page, nav } => Some((page.as_str().to_owned(), nav.clone())),
            _ => None,
        })
        .collect()
}

fn history(session: &mut Scripted, page: &str, back: bool, forward: bool) {
    session.app.page_reports(
        page,
        PageNotice::Loading {
            loading: false,
            can_go_back: back,
            can_go_forward: forward,
        },
    );
}

#[test]
fn reload_navigates_the_named_page_whatever_is_entered_or_selected() {
    let mut session = session();
    session.app.select(&["p2"]).double_click((800.0, 150.0));
    let response = session.post("/pages/p1/reload", json!({}));
    assert_eq!(response.status, 200, "{}", response.body);
    assert_eq!(response.body, json!({ "ok": true, "pageId": "p1" }));
    assert_eq!(navigations(&session), [("p1".to_owned(), PageNav::Reload)]);
}

#[test]
fn back_and_forward_run_where_the_page_can_go_and_are_refused_elsewhere() {
    let mut session = session();
    let refused = session.post("/pages/p1/back", json!({}));
    assert_eq!(refused.status, 409);
    assert!(
        refused.body["error"]
            .as_str()
            .is_some_and(|text| text.contains("go back"))
    );
    assert_eq!(navigations(&session).len(), 0);

    history(&mut session, "p1", true, false);
    assert_eq!(session.post("/pages/p1/back", json!({})).status, 200);
    assert_eq!(navigations(&session), [("p1".to_owned(), PageNav::Back)]);
    let refused = session.post("/pages/p1/forward", json!({}));
    assert_eq!(refused.status, 409);
    assert!(
        refused.body["error"]
            .as_str()
            .is_some_and(|text| text.contains("go forward"))
    );

    history(&mut session, "p1", true, true);
    assert_eq!(session.post("/pages/p1/forward", json!({})).status, 200);
    assert_eq!(navigations(&session), [("p1".to_owned(), PageNav::Forward)]);
}

#[test]
fn an_unknown_page_is_not_found() {
    let mut session = session();
    for verb in ["back", "forward", "reload"] {
        let response = session.post(&format!("/pages/nope/{verb}"), json!({}));
        assert_eq!(response.status, 404, "{verb}");
    }
    assert_eq!(session.get("/pages/nope/cdp-target").status, 404);
}

#[test]
fn the_debugging_target_is_what_the_page_reported() {
    let mut app = TestApp::with_pages(1);
    app.page_reports("p1", PageNotice::Url("https://example.com/p1".to_owned()));
    let mut session = Scripted::new(app);
    let early = session.get("/pages/p1/cdp-target");
    assert_eq!(early.status, 503, "{}", early.body);
    assert!(
        early.body["error"]
            .as_str()
            .is_some_and(|text| text.contains("no debugging target"))
    );

    session.app.page_reports(
        "p1",
        PageNotice::DevtoolsUrl("ws://127.0.0.1:9222/devtools/page/ABC".to_owned()),
    );
    let ready = session.get("/pages/p1/cdp-target");
    assert_eq!(ready.status, 200);
    assert_eq!(
        ready.body,
        json!({
            "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/ABC",
            "url": "https://example.com/p1",
        })
    );
    session.app.page_reports(
        "p1",
        PageNotice::Url("https://example.org/moved".to_owned()),
    );
    assert_eq!(
        session.get("/pages/p1/cdp-target").body["url"],
        "https://example.org/moved"
    );
}

#[test]
fn updating_a_pages_url_navigates_it_in_place() {
    let mut session = session();
    session
        .app
        .page_reports("p1", PageNotice::Title("one".to_owned()));
    let done =
        session.apply(json!({ "entities": [{ "id": "p1", "url": "https://example.org/next" }] }));
    assert_eq!(done["updated"], json!(["p1"]));
    assert_eq!(
        navigations(&session),
        [(
            "p1".to_owned(),
            PageNav::To("https://example.org/next".to_owned())
        )]
    );
    assert!(
        !session
            .effects
            .iter()
            .any(|effect| matches!(effect, Effect::ClosePage(_) | Effect::CreatePage { .. }))
    );
    // The page stays hosted, so what it said of itself is kept.
    assert!(session.app.app().page_state(&"p1".into()).is_some());
    session.undo();
    assert_eq!(
        navigations(&session),
        [(
            "p1".to_owned(),
            PageNav::To("https://example.com/p1".to_owned())
        )]
    );
}

#[test]
fn with_several_pages_the_debugging_target_is_withheld_and_the_socket_named() {
    let mut session = Scripted::new(TestApp::with_pages(2));
    session.app.page_reports(
        "p2",
        PageNotice::DevtoolsUrl("ws://127.0.0.1:9222/devtools/page/TWO".to_owned()),
    );
    let response = session.get("/pages/p2/cdp-target");
    assert_eq!(response.status, 501, "{}", response.body);
    let error = response.body["error"].as_str().unwrap_or_default();
    assert!(error.contains("more than one page"), "{error}");
    assert!(
        error.contains("ws://127.0.0.1:9222/devtools/page/TWO"),
        "{error}"
    );
}
