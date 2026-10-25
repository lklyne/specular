//! The settings that hide agent cursors and the synced cursor. Hiding one
//! stops drawing it and nothing else: the agent's act is still let go, and
//! the peer still gets the replayed click.
//!
//! The change that breaks it: reading the setting where the cursor is moved
//! or the act answered, instead of where the cursor is drawn.

use glam::DVec2;
use specular_core::{LocatorBundle, LocatorCandidate, LocatorRect, PointKind};
use specular_interact::{Action, AgentAct, Effect, Event, PageNotice};
use specular_testkit::TestApp;

/// With the settings `switches` pressed first: p1 entered, an agent's click
/// on it, and p1's click replayed on its sync peer p2. Also says whether
/// the agent was let act and whether p2 was clicked.
fn with_both_cursors(switches: &[&str]) -> (TestApp, bool, bool) {
    let mut app = TestApp::with_pages(2);
    for switch in switches {
        assert!(app.control(switch).is_ok(), "no switch {switch}");
    }
    app.select(&["p1", "p2"]).act(Action::ToggleSync);
    for page in ["p1", "p2"] {
        app.page_reports(page, PageNotice::Url("https://site.test/".to_owned()));
    }
    app.select(&[]).click((200.0, 150.0)).click((200.0, 150.0));
    app.take_effects();
    app.send(Event::AgentAct(AgentAct {
        ticket: Some(7),
        page: "p1".into(),
        session: "agent:p1".to_owned(),
        action: "click".to_owned(),
        at: DVec2::new(40.0, 40.0),
    }));
    app.tick(1_000_000).tick(1_001_000);
    let let_go = app.take_effects().contains(&Effect::AgentGo { ticket: 7 });
    let bundle = LocatorBundle {
        name: Some("button \"Menu\"".to_owned()),
        tag: "button".to_owned(),
        offset_x: 0.5,
        offset_y: 0.5,
        ..LocatorBundle::default()
    };
    app.page_reports(
        "p1",
        PageNotice::Pointed {
            kind: PointKind::Click,
            bundle: Box::new(bundle),
        },
    );
    let request = (app.take_effects().iter())
        .find_map(|effect| match effect {
            Effect::AskCandidates { request, .. } => Some(*request),
            _ => None,
        })
        .unwrap_or_default();
    let menu = LocatorCandidate {
        name: Some("button \"Menu\"".to_owned()),
        tag: Some("button".to_owned()),
        interactive: true,
        rect: LocatorRect {
            x: 300.0,
            y: 20.0,
            width: 40.0,
            height: 20.0,
        },
        ..LocatorCandidate::default()
    };
    app.page_reports(
        "p2",
        PageNotice::Candidates {
            request,
            candidates: vec![menu],
        },
    );
    let replayed = (app.take_effects().iter()).any(
        |effect| matches!(effect, Effect::ReplayPointer { page, .. } if page.as_str() == "p2"),
    );
    (app, let_go, replayed)
}

#[test]
fn a_hidden_cursor_is_not_drawn_and_what_it_stands_for_still_happens() {
    let counts = |app: &TestApp| {
        (
            app.app().agent_cursors().len(),
            app.app().synced_cursors().len(),
        )
    };
    let rows = [
        ("none hidden", vec![], (1, 1)),
        (
            "agent cursors hidden",
            vec!["settings.agent-cursors"],
            (0, 1),
        ),
        (
            "synced cursors hidden",
            vec!["settings.synced-cursors"],
            (1, 0),
        ),
        (
            "both hidden, then shown again",
            vec![
                "settings.agent-cursors",
                "settings.synced-cursors",
                "settings.agent-cursors",
                "settings.synced-cursors",
            ],
            (1, 1),
        ),
    ];
    for (name, switches, drawn) in rows {
        let (app, let_go, replayed) = with_both_cursors(&switches);
        assert!(let_go, "{name}: the agent was not let act");
        assert!(replayed, "{name}: the peer was not clicked");
        assert_eq!(counts(&app), drawn, "{name}");
    }
}
