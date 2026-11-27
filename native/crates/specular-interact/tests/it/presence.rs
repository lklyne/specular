//! Agent cursors: one per agent, sent to what its calls change, resting
//! and fading between them.
//!
//! The change that breaks it: dropping a caller's session when it is not
//! one of the app's own threads leaves the CLI with no cursor, moving the
//! cursor before a change it cannot yet see points at nothing, and keeping
//! a quiet cursor owes frames for ever.

use glam::DVec2;
use specular_doc::{Command, Rect};
use specular_interact::{AgentAct, ApiCall, ApiRun, App, Effect, Event, ThreadId};
use specular_testkit::{TestApp, page, sticky};

const START: u64 = 1_000_000;

fn app() -> TestApp {
    let mut app = TestApp::with_entities([
        sticky("s1", Rect::new(0.0, 0.0, 200.0, 200.0), "one"),
        sticky("s2", Rect::new(0.0, 400.0, 200.0, 200.0), "two"),
    ]);
    app.viewport((1000.0, 800.0)).tick(START);
    app
}

fn call(app: &mut TestApp, agent: Option<&str>, run: ApiRun) {
    app.send(Event::Api(ApiCall {
        ticket: 1,
        canvas: None,
        thread: agent.map(|agent| ThreadId(agent.to_owned())),
        run,
    }));
}

fn apply(command: Command) -> ApiRun {
    ApiRun::Apply {
        command,
        select: None,
    }
}

/// Where the one cursor is and what it says.
fn cursor(app: &App) -> Option<(DVec2, &'static str)> {
    let cursors = app.agent_cursors();
    assert!(cursors.len() <= 1, "one agent, one cursor");
    cursors.first().map(|cursor| (cursor.at, cursor.label))
}

#[test]
fn an_agents_edit_sends_its_cursor_to_what_changed_and_a_nameless_one_shows_none() {
    let mut app = app();
    let added = sticky("s3", Rect::new(400.0, 100.0, 200.0, 200.0), "three");
    call(
        &mut app,
        None,
        apply(Command::InsertEntity {
            entity: Box::new(added.clone()),
            at: 2,
        }),
    );
    app.tick(START + 16);
    assert_eq!(cursor(app.app()), None);

    let moved = Command::SetRect {
        id: "s3".into(),
        rect: Rect::new(600.0, 100.0, 200.0, 200.0),
    };
    call(&mut app, Some("claude-1"), apply(moved));
    app.tick(START + 32).tick(START + 100);
    assert_eq!(
        cursor(app.app()),
        Some((DVec2::new(624.0, 124.0), "moving"))
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_change_to_several_items_visits_each_in_reading_order() {
    let mut app = app();
    let both = Command::Batch(vec![
        Command::SetRect {
            id: "s2".into(),
            rect: Rect::new(300.0, 400.0, 200.0, 200.0),
        },
        Command::SetRect {
            id: "s1".into(),
            rect: Rect::new(300.0, 0.0, 200.0, 200.0),
        },
    ]);
    call(&mut app, Some("claude-1"), apply(both));
    app.tick(START + 16).tick(START + 60);
    assert_eq!(cursor(app.app()), Some((DVec2::new(324.0, 24.0), "moving")));
    // It rests on the first, then goes on.
    app.tick(START + 240).tick(START + 300);
    assert_eq!(
        cursor(app.app()),
        Some((DVec2::new(324.0, 424.0), "moving"))
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn a_quiet_cursor_thinks_then_goes() {
    let mut app = app();
    let edit = Command::SetLabel {
        id: "s1".into(),
        label: Some("first".to_owned()),
    };
    call(&mut app, Some("claude-1"), apply(edit));
    app.tick(START + 16).tick(START + 60);
    assert_eq!(cursor(app.app()).map(|(_, label)| label), Some("editing"));
    let epoch = app.app().agent_cursor_epoch();
    app.tick(START + 3_100);
    assert_eq!(cursor(app.app()).map(|(_, label)| label), Some("thinking"));
    assert_ne!(
        app.app().agent_cursor_epoch(),
        epoch,
        "the new label is drawn"
    );
    app.tick(START + 29_500);
    let fading = app.app().agent_cursors()[0].alpha;
    assert!(fading > 0.0 && fading < 1.0, "fading, at {fading}");
    app.tick(START + 31_000);
    assert_eq!(cursor(app.app()), None);
    assert!(!app.app().agent_cursor_travelling());
    app.assert_undo_returns_to_start();
}

#[test]
fn a_read_sweeps_the_cursor_over_what_is_in_view() {
    let mut app = app();
    let report = |key: &str| ApiRun::Report {
        key: key.to_owned(),
    };
    call(&mut app, Some("claude-1"), report("scan_workspace"));
    app.tick(START + 16).tick(START + 60);
    assert_eq!(cursor(app.app()), Some((DVec2::new(24.0, 24.0), "reading")));
    app.tick(START + 240).tick(START + 300);
    assert_eq!(
        cursor(app.app()),
        Some((DVec2::new(24.0, 424.0), "reading"))
    );
    // The agent says it is done: the cursor fades out.
    call(&mut app, Some("claude-1"), report("departing"));
    app.tick(START + 1_400);
    assert_eq!(cursor(app.app()), None);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_click_waits_for_the_cursor_and_lands_with_a_ripple() {
    let mut app = TestApp::with_entities([page("p1", Rect::new(0.0, 0.0, 800.0, 600.0))]);
    app.viewport((1000.0, 800.0)).tick(START);
    app.with_motion();
    app.take_effects();
    app.send(Event::AgentAct(AgentAct {
        ticket: Some(7),
        page: "p1".into(),
        session: "claude-1:p1".to_owned(),
        action: "click".to_owned(),
        at: DVec2::new(400.0, 300.0),
    }));
    let go = |effects: Vec<Effect>| effects.contains(&Effect::AgentGo { ticket: 7 });
    assert!(!go(app.take_effects()), "the click waits for the cursor");
    app.tick(START + 16);
    assert!(!go(app.take_effects()));
    assert!(app.app().agent_cursor_travelling());
    app.tick(START + 1_000);
    assert!(go(app.take_effects()), "the cursor has landed");
    app.tick(START + 1_100);
    let landed = &app.app().agent_cursors()[0];
    assert_eq!(landed.label, "clicking");
    assert!(landed.ripple.is_some() && landed.trail.is_empty());
    app.assert_undo_returns_to_start();
}
