//! Sync sets (ADR 0027): the chain button's toggle-merge, and a set's
//! pages following each other's navigations and the entered page's scroll,
//! hovers and clicks (ADR 0030).

#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use glam::Vec2;
use specular_core::{LocatorBundle, LocatorCandidate, LocatorRect, PointKind};
use specular_doc::{EntityId, Kind};
use specular_interact::{Action, Effect, Key, PageNotice};
use specular_testkit::TestApp;

/// Inside p1 of `TestApp::with_pages(n)`.
const ON_P1: (f32, f32) = (200.0, 150.0);

fn set_of(app: &TestApp, name: &str) -> Option<String> {
    match &app.entity(name).kind {
        Kind::Page(page) => page.sync_id.clone(),
        other => panic!("{name} is not a page: {other:?}"),
    }
}

/// Three pages with p1 and p2 in one sync set, each having reported the
/// address it loaded, and no effects pending.
fn synced_pair() -> TestApp {
    let mut app = TestApp::with_pages(3);
    app.select(&["p1", "p2"]).act(Action::ToggleSync);
    for page in ["p1", "p2", "p3"] {
        app.page_reports(page, PageNotice::Url("https://site.test/".to_owned()));
    }
    app.select(&[]);
    app.take_effects();
    app
}

/// What the step since the last drain asked of pages: navigations and
/// scrolls, by page.
fn asked(app: &mut TestApp) -> Vec<String> {
    (app.take_effects().iter())
        .filter_map(|effect| match effect {
            Effect::Navigate { page, nav } => Some(format!("{} {nav:?}", page.as_str())),
            Effect::AskScrollProgress(page) => Some(format!("{} progress?", page.as_str())),
            Effect::ScrollPage { page, progress } => {
                Some(format!("{} scroll {progress}", page.as_str()))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_chain_button_merges_the_selected_pages_into_one_set_and_takes_one_set_apart() {
    let mut app = TestApp::with_pages(4);
    app.with_panels();

    // Two loose pages join a new set, and the button reads as on.
    app.select(&["p1", "p2"]).click_control("page.sync");
    let first = set_of(&app, "p1").expect("p1 joined a set");
    assert!(first.starts_with("sync_"));
    assert_eq!(set_of(&app, "p2").as_deref(), Some(first.as_str()));
    assert_eq!(app.app().selection_synced(), Some(true));

    // A selection straddling a set and a loose page merges into a new set;
    // p1, left alone in the old one, is no longer in any.
    app.select(&["p2", "p3"]).click_control("page.sync");
    let second = set_of(&app, "p2").expect("p2 joined a set");
    assert_ne!(second, first);
    assert_eq!(set_of(&app, "p3").as_deref(), Some(second.as_str()));
    assert_eq!(set_of(&app, "p1"), None);

    // One undo puts the first set back whole.
    app.undo();
    assert_eq!(set_of(&app, "p1").as_deref(), Some(first.as_str()));
    assert_eq!(set_of(&app, "p2").as_deref(), Some(first.as_str()));
    assert_eq!(set_of(&app, "p3"), None);

    // A whole set selected leaves it, and so does one page of a set.
    app.select(&["p1", "p2"]).click_control("page.sync");
    assert_eq!((set_of(&app, "p1"), set_of(&app, "p2")), (None, None));
    app.undo().select(&["p1"]).click_control("page.sync");
    assert_eq!((set_of(&app, "p1"), set_of(&app, "p2")), (None, None));

    // A page in no set has no chain button, and the action does nothing.
    app.select(&["p4"]);
    assert_eq!(app.app().selection_synced(), None);
    let before = app.document().clone();
    app.act(Action::ToggleSync);
    assert_eq!(*app.document(), before);

    app.assert_undo_returns_to_start();
}

#[test]
fn a_page_that_navigates_takes_its_sync_set_there_and_no_arrival_echoes_back() {
    let mut app = synced_pair();
    let to = |url: &str| format!("To({url:?})");

    // A link followed in p1 sends p2 there, and not p3, which is unsynced.
    app.page_reports("p1", PageNotice::Url("https://site.test/a".to_owned()));
    assert_eq!(
        asked(&mut app),
        [format!("p2 {}", to("https://site.test/a"))]
    );

    // p2 arriving, and being redirected on the way, sends nothing back.
    app.page_reports("p2", PageNotice::Url("https://site.test/a".to_owned()));
    app.page_reports("p2", PageNotice::Url("https://m.site.test/a".to_owned()));
    assert_eq!(asked(&mut app), [] as [String; 0]);

    // Once the load has settled, p2 leads as well as follows.
    app.tick(2_000);
    app.page_reports("p2", PageNotice::Url("https://site.test/b".to_owned()));
    assert_eq!(
        asked(&mut app),
        [format!("p1 {}", to("https://site.test/b"))]
    );
    app.page_reports("p1", PageNotice::Url("https://site.test/b".to_owned()));

    // A change of hash is followed like any address.
    app.tick(4_000);
    app.page_reports("p1", PageNotice::Url("https://site.test/b#x".to_owned()));
    assert_eq!(
        asked(&mut app),
        [format!("p2 {}", to("https://site.test/b#x"))]
    );

    // A page reporting an address its peers already show sends nobody.
    app.tick(6_000);
    app.page_reports("p2", PageNotice::Url("https://site.test/b#x".to_owned()));
    assert_eq!(asked(&mut app), [] as [String; 0]);
}

#[test]
fn the_history_buttons_and_the_address_field_drive_the_whole_sync_set() {
    let mut app = synced_pair();
    let history = |back| PageNotice::Loading {
        loading: false,
        can_go_back: back,
        can_go_forward: false,
    };
    app.page_reports("p1", history(true));
    app.page_reports("p2", history(true));
    app.select(&["p1"]);
    app.take_effects();

    let rows: [(Action, &[&str]); 3] = [
        (Action::PageBack, &["p1 Back", "p2 Back"]),
        (Action::PageReload, &["p1 Reload", "p2 Reload"]),
        (
            Action::PageNavigate("https://site.test/c".to_owned()),
            &[
                "p1 To(\"https://site.test/c\")",
                "p2 To(\"https://site.test/c\")",
            ],
        ),
    ];
    for (action, expected) in rows {
        app.act(action.clone());
        assert_eq!(asked(&mut app), expected, "{action:?}");
    }

    // A peer with nowhere to go back to is loaded where the page was.
    app.page_reports("p2", history(false));
    app.act(Action::PageBack);
    assert_eq!(
        asked(&mut app),
        ["p1 Back", "p2 To(\"https://site.test/\")"]
    );

    // What the driven pages then report is not sent round again.
    app.page_reports("p1", PageNotice::Url("https://site.test/z".to_owned()));
    assert_eq!(asked(&mut app), [] as [String; 0]);
}

#[test]
fn the_entered_pages_scroll_moves_its_sync_set_to_the_same_fraction() {
    let mut app = synced_pair();
    let scrolled = PageNotice::Scrolled { x: 0.0, y: 300.0 };

    // A page that is not entered scrolled by itself: nobody follows.
    app.page_reports("p1", scrolled.clone());
    assert_eq!(asked(&mut app), [] as [String; 0]);

    // The entered page asks how far through its document it is, and the
    // answer goes to its peers as a fraction, so each lands at the same
    // place in a document of its own height.
    app.click(ON_P1).click(ON_P1);
    app.take_effects();
    app.page_reports("p1", scrolled);
    assert_eq!(asked(&mut app), ["p1 progress?"]);
    app.page_reports("p1", PageNotice::ScrollProgress { x: 0.0, y: 0.5 });
    assert_eq!(
        asked(&mut app),
        [format!("p2 scroll {}", Vec2::new(0.0, 0.5))]
    );

    // The follower's own scroll report goes nowhere.
    app.page_reports("p2", PageNotice::Scrolled { x: 0.0, y: 450.0 });
    app.page_reports("p2", PageNotice::ScrollProgress { x: 0.0, y: 0.5 });
    assert_eq!(asked(&mut app), [] as [String; 0]);
}

fn button(id: Option<&str>, name: &str, x: f64) -> LocatorCandidate {
    LocatorCandidate {
        id: id.map(str::to_owned),
        name: Some(name.to_owned()),
        tag: Some("button".to_owned()),
        interactive: true,
        rect: LocatorRect {
            x,
            y: 20.0,
            width: 40.0,
            height: 20.0,
        },
        ..LocatorCandidate::default()
    }
}

/// The request of the one candidates question the last step asked `page`.
fn asked_candidates(app: &mut TestApp, page: &str) -> Option<u64> {
    let effects = app.take_effects();
    let mut asked = effects.iter().filter_map(|effect| match effect {
        Effect::AskCandidates {
            page: peer,
            request,
            ..
        } if peer.as_str() == page => Some(*request),
        _ => None,
    });
    asked.next().filter(|_| asked.next().is_none())
}

fn replays(app: &mut TestApp) -> Vec<(String, PointKind, Vec2)> {
    (app.take_effects().into_iter())
        .filter_map(|effect| match effect {
            Effect::ReplayPointer { page, kind, point } => {
                Some((page.as_str().to_owned(), kind, point))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn the_entered_pages_hovers_and_clicks_replay_on_a_peers_own_element_or_not_at_all() {
    let mut app = synced_pair();
    let pointed = |kind| PageNotice::Pointed {
        kind,
        bundle: Box::new(LocatorBundle {
            name: Some("button \"Menu\"".to_owned()),
            tag: "button".to_owned(),
            offset_x: 0.5,
            offset_y: 0.5,
            ..LocatorBundle::default()
        }),
    };
    let answer = |request, candidates| PageNotice::Candidates {
        request,
        candidates,
    };

    // Nothing is captured until a page of a sync set is entered.
    app.page_reports("p1", pointed(PointKind::Click));
    assert_eq!(asked_candidates(&mut app, "p2"), None);
    app.click(ON_P1).click(ON_P1);
    let captured = app.take_effects();
    assert!(captured.contains(&Effect::CapturePage(Some(EntityId::new("p1")))));

    // A click on the entered page asks its peer, and lands on the peer's
    // own button, wherever that page's layout put it.
    app.page_reports("p1", pointed(PointKind::Click));
    let request = asked_candidates(&mut app, "p2").expect("p2 was asked");
    let menu_at_300 = vec![
        button(None, "button \"Menu\"", 300.0),
        button(None, "button \"Buy\"", 10.0),
    ];
    app.page_reports("p2", answer(request, menu_at_300.clone()));
    assert_eq!(
        replays(&mut app),
        [("p2".to_owned(), PointKind::Click, Vec2::new(320.0, 30.0))]
    );

    // Each row is what a peer answers and what is replayed on it.
    let two_menus = vec![
        button(None, "button \"Menu\"", 300.0),
        button(None, "button \"Menu\"", 500.0),
    ];
    let rows: [(&str, PointKind, Vec<LocatorCandidate>, usize); 4] = [
        (
            "an ambiguous click is refused",
            PointKind::Click,
            two_menus,
            0,
        ),
        (
            "a missing element is refused",
            PointKind::Click,
            Vec::new(),
            0,
        ),
        (
            "a hover is replayed as a hover",
            PointKind::Hover,
            menu_at_300.clone(),
            1,
        ),
        ("a click again", PointKind::Click, menu_at_300.clone(), 1),
    ];
    for (name, kind, candidates, replayed) in rows {
        app.page_reports("p1", pointed(kind));
        let request = asked_candidates(&mut app, "p2").expect(name);
        app.page_reports("p2", answer(request, candidates));
        let replays = replays(&mut app);
        assert_eq!(replays.len(), replayed, "{name}");
        assert!(
            replays.iter().all(|(_, replayed, _)| *replayed == kind),
            "{name}"
        );
    }

    // An answer to a question a newer one replaced is dropped.
    app.page_reports("p1", pointed(PointKind::Click));
    let stale = asked_candidates(&mut app, "p2").expect("p2 was asked");
    app.page_reports("p1", pointed(PointKind::Click));
    app.take_effects();
    app.page_reports("p2", answer(stale, menu_at_300.clone()));
    assert_eq!(replays(&mut app), []);

    // A link clicked on both pages loads the peer once: the peer follows
    // its own click, and is sent there only if that click took it nowhere.
    app.page_reports("p1", pointed(PointKind::Click));
    let request = asked_candidates(&mut app, "p2").expect("p2 was asked");
    app.page_reports("p2", answer(request, menu_at_300.clone()));
    app.page_reports("p1", PageNotice::Url("https://site.test/next".to_owned()));
    assert_eq!(asked(&mut app), [] as [String; 0]);
    app.tick(2_000);
    assert_eq!(asked(&mut app), ["p2 To(\"https://site.test/next\")"]);
    app.page_reports("p2", PageNotice::Url("https://site.test/next".to_owned()));
    app.tick(5_000);
    app.take_effects();

    // What a peer is pointed at is replayed input, and is not sent on.
    app.page_reports("p2", pointed(PointKind::Click));
    assert_eq!(asked_candidates(&mut app, "p1"), None);

    // A peer on another origin is not asked.
    app.tick(20_000);
    app.page_reports("p2", PageNotice::Url("https://other.test/".to_owned()));
    app.take_effects();
    app.page_reports("p1", pointed(PointKind::Click));
    assert_eq!(asked_candidates(&mut app, "p2"), None);

    // Leaving the page ends the capture.
    app.key(Key::Escape);
    assert!(app.take_effects().contains(&Effect::CapturePage(None)));
}
