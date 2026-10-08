//! What a hosted page reports of itself, and the browser verbs that act on
//! it: the notices land in the app's page state, the address is saved
//! without a history step, and a changed address navigates in place.

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_core::PageNav;
use specular_doc::{Command, EntityId, Kind, Page};
use specular_interact::{Action, ApiCall, ApiRun, Effect, Event, Focus, Key, PageNotice};
use specular_testkit::{CMD, TestApp};

/// Inside p1 of `TestApp::with_pages(n)`.
const ON_P1: (f32, f32) = (200.0, 150.0);

fn id(name: &str) -> EntityId {
    EntityId::new(name)
}

fn loading(loading: bool, back: bool, forward: bool) -> PageNotice {
    PageNotice::Loading {
        loading,
        can_go_back: back,
        can_go_forward: forward,
    }
}

fn navigations(effects: &[Effect]) -> Vec<(&EntityId, &PageNav)> {
    (effects.iter())
        .filter_map(|effect| match effect {
            Effect::Navigate { page, nav } => Some((page, nav)),
            _ => None,
        })
        .collect()
}

fn url_of(app: &TestApp, name: &str) -> String {
    match &app.entity(name).kind {
        Kind::Page(page) => page.url.clone(),
        other => panic!("{name} is not a page: {other:?}"),
    }
}

/// Changes a page's address the way the API's `update --url` does.
fn set_url(app: &mut TestApp, name: &str, url: &str) {
    let Kind::Page(current) = &app.entity(name).kind else {
        panic!("{name} is not a page");
    };
    let kind = Kind::Page(Page {
        url: url.to_owned(),
        ..current.clone()
    });
    let command = Command::SetKind {
        id: id(name),
        kind: Box::new(kind),
    };
    app.send(Event::Api(ApiCall {
        ticket: 1,
        canvas: None,
        run: ApiRun::Apply {
            command,
            select: None,
        },
    }));
}

#[test]
fn each_notice_lands_in_the_page_state() {
    let mut app = TestApp::with_pages(2);
    assert_eq!(app.app().page_state(&id("p1")), None);
    app.page_reports("p1", PageNotice::Title("Example Domain".to_owned()))
        .page_reports("p1", loading(true, true, false))
        .page_reports("p1", PageNotice::Scrolled { x: 0.0, y: 240.5 })
        .page_reports(
            "p1",
            PageNotice::DevtoolsUrl("ws://127.0.0.1:9222/x".into()),
        );
    let state = app.app().page_state(&id("p1")).expect("p1 reported");
    assert_eq!(state.title, "Example Domain");
    assert!(state.loading && state.can_go_back && !state.can_go_forward);
    assert_eq!(state.devtools_url.as_deref(), Some("ws://127.0.0.1:9222/x"));
    assert_eq!(
        app.app().page_scroll(&id("p1")),
        glam::DVec2::new(0.0, 240.5)
    );
    // The other page heard nothing.
    assert_eq!(app.app().page_state(&id("p2")), None);
    assert_eq!(app.app().page_scroll(&id("p2")), glam::DVec2::ZERO);
    app.page_reports("p1", loading(false, true, true));
    assert_eq!(
        app.app()
            .page_state(&id("p1"))
            .map(|s| (s.loading, s.can_go_forward)),
        Some((false, true))
    );
}

#[test]
fn a_new_address_is_saved_without_a_history_step() {
    let mut app = TestApp::with_pages(1);
    assert!(!app.app().can_undo());
    app.take_effects();
    app.page_reports("p1", PageNotice::Url("https://example.org/next".to_owned()));
    assert_eq!(url_of(&app, "p1"), "https://example.org/next");
    assert_eq!(
        app.app()
            .page_state(&id("p1"))
            .and_then(|s| s.url.as_deref()),
        Some("https://example.org/next")
    );
    assert!(!app.app().can_undo(), "the address is not an undo step");
    assert_eq!(app.take_effects(), [Effect::Save]);
    // The same address again changes nothing and saves nothing.
    app.page_reports("p1", PageNotice::Url("https://example.org/next".to_owned()));
    assert_eq!(app.take_effects(), []);
    // The page's own address is not an instruction to navigate.
    app.page_reports(
        "p1",
        PageNotice::Url("https://example.org/other".to_owned()),
    );
    assert_eq!(navigations(&app.take_effects()).len(), 0);
}

#[test]
fn a_changed_address_navigates_in_place_and_undo_navigates_back() {
    let mut app = TestApp::with_pages(1);
    app.double_click(ON_P1);
    assert_eq!(app.session().focus, Focus::Page(id("p1")));
    app.take_effects();
    set_url(&mut app, "p1", "https://example.org/");
    let effects = app.take_effects();
    assert_eq!(
        navigations(&effects),
        [(&id("p1"), &PageNav::To("https://example.org/".to_owned()))]
    );
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::ClosePage(_) | Effect::CreatePage { .. }))
    );
    assert_eq!(app.session().focus, Focus::Page(id("p1")), "still entered");
    app.undo();
    let effects = app.take_effects();
    assert_eq!(
        navigations(&effects),
        [(&id("p1"), &PageNav::To("https://example.com/p1".to_owned()))]
    );
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::ClosePage(_) | Effect::CreatePage { .. }))
    );
    assert_eq!(app.session().focus, Focus::Page(id("p1")));
    app.assert_undo_returns_to_start();
}

#[test]
fn back_and_forward_wait_for_the_page_to_say_there_is_somewhere_to_go() {
    let mut app = TestApp::with_pages(1);
    app.select(&["p1"]).take_effects();
    app.act(Action::PageBack).act(Action::PageForward);
    assert_eq!(app.take_effects(), []);
    app.page_reports("p1", loading(false, true, false));
    app.act(Action::PageBack);
    assert_eq!(
        navigations(&app.take_effects()),
        [(&id("p1"), &PageNav::Back)]
    );
    app.act(Action::PageForward);
    assert_eq!(app.take_effects(), []);
    app.page_reports("p1", loading(false, true, true));
    app.act(Action::PageForward);
    assert_eq!(
        navigations(&app.take_effects()),
        [(&id("p1"), &PageNav::Forward)]
    );
}

#[test]
fn stop_only_while_loading_and_reload_always() {
    let mut app = TestApp::with_pages(1);
    app.select(&["p1"]).take_effects();
    app.act(Action::PageStop);
    assert_eq!(app.take_effects(), []);
    app.act(Action::PageReload);
    assert_eq!(
        navigations(&app.take_effects()),
        [(&id("p1"), &PageNav::Reload)]
    );
    app.page_reports("p1", loading(true, false, false));
    app.act(Action::PageStop);
    assert_eq!(
        navigations(&app.take_effects()),
        [(&id("p1"), &PageNav::Stop)]
    );
    app.page_reports("p1", loading(false, false, false));
    app.act(Action::PageStop);
    assert_eq!(app.take_effects(), []);
}

#[test]
fn the_target_is_the_entered_page_else_the_single_selected_page() {
    let mut app = TestApp::with_pages(2);
    app.take_effects();
    app.act(Action::PageReload);
    assert_eq!(app.take_effects(), [], "nothing selected");
    app.select(&["p1", "p2"]).act(Action::PageReload);
    assert_eq!(app.take_effects(), [], "two selected");
    app.select(&["p2"]).act(Action::PageReload);
    assert_eq!(
        navigations(&app.take_effects()),
        [(&id("p2"), &PageNav::Reload)]
    );
    // Entering p1 selects it too, so the target moves to it with the entry.
    app.double_click(ON_P1);
    assert_eq!(app.session().focus, Focus::Page(id("p1")));
    app.take_effects();
    app.act(Action::PageReload);
    assert_eq!(
        navigations(&app.take_effects()),
        [(&id("p1"), &PageNav::Reload)]
    );
}

#[test]
fn cmd_bracket_walks_an_entered_pages_history_and_restacks_a_selected_one() {
    let mut app = TestApp::with_pages(2);
    app.page_reports("p1", loading(false, true, true));
    app.select(&["p1"]).take_effects();
    // Merely selected: the canvas keeps the chord.
    let order = |app: &TestApp| -> Vec<String> {
        (app.document().entities())
            .map(|e| e.id.as_str().to_owned())
            .collect()
    };
    app.chord(CMD, Key::Char('['));
    assert_eq!(navigations(&app.take_effects()), []);
    assert_eq!(order(&app), ["p1", "p2"], "p1 is already at the back");
    app.select(&["p2"]).chord(CMD, Key::Char('['));
    assert_eq!(order(&app), ["p2", "p1"], "send backward restacks");
    assert_eq!(navigations(&app.take_effects()), []);
    app.select(&["p1"]);
    app.undo();
    // Entered: the page's history.
    app.double_click(ON_P1);
    app.take_effects();
    app.chord(CMD, Key::Char('['));
    let effects = app.take_effects();
    assert_eq!(navigations(&effects), [(&id("p1"), &PageNav::Back)]);
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::ForwardInput { .. }))
    );
    app.chord(CMD, Key::Char(']'));
    assert_eq!(
        navigations(&app.take_effects()),
        [(&id("p1"), &PageNav::Forward)]
    );
}

#[test]
fn cmd_r_reloads_a_selected_or_entered_page_and_is_not_the_pages() {
    let mut app = TestApp::with_pages(1);
    app.select(&["p1"]).take_effects();
    app.chord(CMD, Key::Char('r'));
    assert_eq!(
        navigations(&app.take_effects()),
        [(&id("p1"), &PageNav::Reload)]
    );
    app.double_click(ON_P1);
    app.take_effects();
    app.chord(CMD, Key::Char('r'));
    let effects = app.take_effects();
    assert_eq!(navigations(&effects), [(&id("p1"), &PageNav::Reload)]);
    assert!(
        !effects
            .iter()
            .any(|e| matches!(e, Effect::ForwardInput { .. }))
    );
}

#[test]
fn another_document_starts_at_its_top_and_a_hash_change_keeps_the_scroll() {
    let mut app = TestApp::with_pages(1);
    let url = |url: &str| PageNotice::Url(url.to_owned());
    app.page_reports("p1", url("https://a.test/docs"));
    app.page_reports("p1", PageNotice::Scrolled { x: 0.0, y: 240.0 });
    app.page_reports("p1", url("https://a.test/docs#install"));
    assert_eq!(app.app().page_scroll(&"p1".into()).y, 240.0);
    app.page_reports("p1", url("https://a.test/other"));
    assert_eq!(app.app().page_scroll(&"p1".into()).y, 0.0);
}
