//! The page routes: history verbs, the debugging target, and a changed
//! address navigating the page in place.

use serde_json::json;
use specular_api::ShotArea;
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
fn every_page_has_its_own_debugging_target_however_many_there_are() {
    let mut session = Scripted::new(TestApp::with_pages(3));
    session.app.page_reports(
        "p2",
        PageNotice::Url("https://example.org/moved".to_owned()),
    );
    session
        .app
        .page_reports("p2", PageNotice::Title("Moved".to_owned()));
    let second = session.get("/pages/p2/cdp-target");
    assert_eq!(second.status, 200, "{}", second.body);
    assert_eq!(
        second.body,
        json!({
            "pageId": "p2",
            "targetId": "p2",
            "webSocketDebuggerUrl": "ws://127.0.0.1:1/cdp/page/token-p2",
            "url": "https://example.org/moved",
            "title": "Moved",
            "generation": 0,
            "lastSnapshotGeneration": null,
        })
    );
    // A page that has reported nothing is named by the address it was given.
    let third = session.get("/pages/p3/cdp-target");
    assert_eq!(third.body["url"], "https://example.com/p3");
    assert_ne!(
        third.body["webSocketDebuggerUrl"],
        second.body["webSocketDebuggerUrl"]
    );
    assert_eq!(
        session.post("/pages/p1/snapshot-seen", json!({})).status,
        200
    );
    assert_eq!(
        session.post("/pages/nope/snapshot-seen", json!({})).status,
        404
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
fn a_page_screenshot_is_the_named_page_or_the_selected_one() {
    let mut session = session();
    let named = session.post("/pages/screenshot", json!({ "pageId": "p2" }));
    assert_eq!(named.status, 200, "{}", named.body);
    assert_eq!(named.body["mimeType"], "image/png");
    assert!(named.body["base64"].is_string());
    let page = |id: &str, chrome, padding| ShotArea::Page {
        page: id.into(),
        chrome,
        padding,
    };
    assert_eq!(session.shots[0].area, page("p2", false, 0.0));

    let nothing = session.post("/pages/screenshot", json!({}));
    assert_eq!(nothing.status, 400, "{}", nothing.body);
    session.app.select(&["p1"]);
    assert_eq!(session.post("/pages/screenshot", json!({})).status, 200);
    assert_eq!(session.shots[1].area, page("p1", false, 0.0));

    let composite = json!({ "pageId": "p1", "padding": 8 });
    assert_eq!(
        session
            .post("/pages/screenshot-composite", composite)
            .status,
        200
    );
    assert_eq!(session.shots[2].area, page("p1", true, 8.0));
    let missing = session.post("/pages/screenshot", json!({ "pageId": "nope" }));
    assert_eq!(missing.status, 404);
}
