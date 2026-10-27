//! Sync sets (ADR 0027): the timing a set's pages follow each other by,
//! with the page's answers given by request: a navigation and its arrival,
//! the history buttons, and the entered page's hovers and clicks (ADR
//! 0030). The chain button, copies and the scroll are in
//! `fixtures/scenarios/sync-sets.txt`, and the whole of it on real pages in
//! `cef/sync.txt`.

use glam::Vec2;
use specular_core::{LocatorBundle, LocatorCandidate, LocatorRect, PointKind};
use specular_doc::EntityId;
use specular_interact::{Action, Effect, Key, PageNotice};
use specular_testkit::TestApp;

/// Inside p1 of `TestApp::with_pages(n)`.
const ON_P1: (f32, f32) = (200.0, 150.0);

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

    // Still within the quiet after the first arrival, a move is a redirect.
    app.tick(1_000);
    app.page_reports("p2", PageNotice::Url("https://m2.site.test/a".to_owned()));
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

    // A page reporting the address it already showed is not a move, even
    // when a peer has gone elsewhere since.
    app.tick(8_000);
    app.page_reports("p2", PageNotice::Url("https://site.test/c".to_owned()));
    app.take_effects();
    app.tick(12_000);
    app.page_reports("p1", PageNotice::Url("https://site.test/b#x".to_owned()));
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

    // A peer with nowhere to go back to waits for where the page lands, and
    // is loaded there.
    app.page_reports("p2", history(false));
    app.act(Action::PageBack);
    assert_eq!(asked(&mut app), ["p1 Back"]);
    app.page_reports("p1", PageNotice::Url("https://site.test/z".to_owned()));
    assert_eq!(asked(&mut app), [] as [String; 0]);
    app.tick(1_000);
    assert_eq!(asked(&mut app), ["p2 To(\"https://site.test/z\")"]);

    // A peer whose own step back landed somewhere else is put right, and
    // one that landed where the page did is left alone.
    app.tick(4_000);
    app.page_reports("p2", PageNotice::Url("https://site.test/z".to_owned()));
    app.page_reports("p2", history(true));
    app.take_effects();
    let rows: [(u64, &str, &[&str]); 2] = [
        (
            5_000,
            "https://site.test/elsewhere",
            &["p2 To(\"https://site.test/y\")"],
        ),
        (8_600, "https://site.test/y", &[]),
    ];
    for (at, landed, expected) in rows {
        app.tick(at);
        app.page_reports("p1", PageNotice::Url("https://site.test/x".to_owned()));
        app.page_reports("p2", PageNotice::Url("https://site.test/x".to_owned()));
        app.take_effects();
        app.act(Action::PageBack);
        assert_eq!(asked(&mut app), ["p1 Back", "p2 Back"]);
        app.page_reports("p1", PageNotice::Url("https://site.test/y".to_owned()));
        app.page_reports("p2", PageNotice::Url(landed.to_owned()));
        app.tick(at + 500);
        assert_eq!(asked(&mut app), expected, "{landed}");
        app.page_reports("p2", PageNotice::Url("https://site.test/y".to_owned()));
        app.take_effects();
    }

    // A peer already at the address is not sent there again.
    app.tick(10_000);
    app.page_reports("p2", PageNotice::Url("https://site.test/z".to_owned()));
    app.take_effects();
    app.tick(15_000);
    app.act(Action::PageNavigate("https://site.test/z".to_owned()));
    assert_eq!(asked(&mut app), ["p1 To(\"https://site.test/z\")"]);

    // A reload reloads only a peer showing the same address; one showing
    // another is loaded where the page is.
    app.tick(20_000);
    app.page_reports("p2", PageNotice::Url("https://site.test/other".to_owned()));
    app.take_effects();
    app.tick(30_000);
    app.act(Action::PageReload);
    assert_eq!(
        asked(&mut app),
        ["p1 Reload", "p2 To(\"https://site.test/y\")"]
    );
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
#[expect(
    clippy::too_many_lines,
    reason = "one scripted session: each step builds on the state the last left"
)]
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

    // A click outranks the hover before it: the hover's late answer is
    // dropped, the click's lands.
    app.page_reports("p1", pointed(PointKind::Hover));
    let hover = asked_candidates(&mut app, "p2").expect("p2 was asked");
    app.page_reports("p1", pointed(PointKind::Click));
    let click = asked_candidates(&mut app, "p2").expect("p2 was asked");
    app.page_reports("p2", answer(hover, menu_at_300.clone()));
    assert_eq!(replays(&mut app), []);
    app.page_reports("p2", answer(click, menu_at_300.clone()));
    assert_eq!(replays(&mut app).len(), 1);

    // An answer to a question a newer one replaced is dropped.
    app.page_reports("p1", pointed(PointKind::Click));
    let stale = asked_candidates(&mut app, "p2").expect("p2 was asked");
    app.page_reports("p1", pointed(PointKind::Click));
    app.take_effects();
    app.page_reports("p2", answer(stale, menu_at_300.clone()));
    assert_eq!(replays(&mut app), []);

    // A replayed hover does not hold the peer back: it cannot be following
    // a link it did not click.
    app.tick(5_000);
    app.page_reports("p1", pointed(PointKind::Hover));
    let request = asked_candidates(&mut app, "p2").expect("p2 was asked");
    app.page_reports("p2", answer(request, menu_at_300.clone()));
    app.take_effects();
    app.page_reports("p1", PageNotice::Url("https://site.test/hover".to_owned()));
    assert_eq!(asked(&mut app), ["p2 To(\"https://site.test/hover\")"]);
    app.page_reports("p2", PageNotice::Url("https://site.test/hover".to_owned()));
    app.tick(7_000);

    // A link clicked on both pages loads the peer once: the peer follows
    // its own click, and is sent there only if that click took it nowhere.
    app.page_reports("p1", pointed(PointKind::Click));
    let request = asked_candidates(&mut app, "p2").expect("p2 was asked");
    app.page_reports("p2", answer(request, menu_at_300.clone()));
    app.page_reports("p1", PageNotice::Url("https://site.test/next".to_owned()));
    assert_eq!(asked(&mut app), [] as [String; 0]);
    app.tick(7_500);
    assert_eq!(asked(&mut app), [] as [String; 0], "still given time");
    app.tick(9_000);
    assert_eq!(asked(&mut app), ["p2 To(\"https://site.test/next\")"]);
    app.page_reports("p2", PageNotice::Url("https://site.test/next".to_owned()));
    app.tick(12_000);
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

    // Leaving the page ends the capture, and a question asked before is
    // forgotten with it.
    app.tick(30_000);
    app.page_reports("p2", PageNotice::Url("https://site.test/next".to_owned()));
    app.page_reports("p1", pointed(PointKind::Click));
    let forgotten = asked_candidates(&mut app, "p2").expect("p2 was asked");
    app.key(Key::Escape);
    assert!(app.take_effects().contains(&Effect::CapturePage(None)));
    app.page_reports("p2", answer(forgotten, menu_at_300));
    assert_eq!(replays(&mut app), []);

    // A page in no set is not captured.
    app.click((1400.0, 250.0)).click((1400.0, 250.0));
    let entered = app.take_effects();
    assert!(
        !entered
            .iter()
            .any(|effect| matches!(effect, Effect::CapturePage(Some(_)))),
        "{entered:?}"
    );
}
