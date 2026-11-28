//! Agent cursors: one per agent, sent to what its calls change, resting
//! and fading between them.
//!
//! The change that breaks it: dropping a caller's session when it is not
//! one of the app's own threads leaves the CLI with no cursor, moving the
//! cursor before a change it cannot yet see points at nothing, and keeping
//! a quiet cursor owes frames for ever.

use glam::DVec2;
use specular_core::Modifiers;
use specular_doc::{Command, Rect};
use specular_interact::{
    Action, AgentAct, ApiCall, ApiRun, App, Effect, Event, Key, ThreadId, WheelInput,
};
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
        name: None,
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

/// Twelve stickies in a grid, all in view, and an agent's cursor on the
/// first after a read.
fn crowd() -> TestApp {
    let notes = (0..12).map(|n| {
        let (x, y) = (f64::from(n % 4) * 240.0, f64::from(n / 4) * 240.0);
        sticky(&format!("n{n}"), Rect::new(x, y, 200.0, 200.0), "note")
    });
    let mut app = TestApp::with_entities(notes);
    app.viewport((1200.0, 900.0)).tick(START);
    app
}

#[test]
fn a_read_lights_each_item_it_reads_and_crosses_the_whole_view() {
    let mut app = crowd();
    let report = ApiRun::Report {
        key: "scan_workspace".to_owned(),
        name: None,
    };
    call(&mut app, Some("claude-1"), report);
    let mut lit = std::collections::BTreeSet::new();
    for step in 1..200 {
        app.tick(START + step * 10);
        for (rect, _) in &app.app().agent_cursors()[0].read {
            lit.insert((rect.x as i64, rect.y as i64));
        }
    }
    assert_eq!(
        lit.len(),
        12,
        "every item in view is read, and lit as it is"
    );
    assert!(lit.contains(&(720, 480)), "the last row is reached");
    app.assert_undo_returns_to_start();
}

#[test]
fn the_agents_key_names_each_agent_and_the_one_followed() {
    let mut app = app();
    let named = ApiRun::Report {
        key: "open".to_owned(),
        name: Some("Design bot".to_owned()),
    };
    call(&mut app, Some("bot"), named);
    let edit = Command::SetLabel {
        id: "s2".into(),
        label: Some("b".to_owned()),
    };
    call(&mut app, Some("bot"), apply(edit));
    let other = Command::SetLabel {
        id: "s1".into(),
        label: Some("a".to_owned()),
    };
    call(&mut app, Some("claude-1"), apply(other));
    app.tick(START + 16).tick(START + 60);
    let names: Vec<String> = app
        .app()
        .agents()
        .into_iter()
        .map(|badge| badge.name)
        .collect();
    assert_eq!(names, ["Design bot", "Agent 2"]);
    app.act(Action::Follow(Some("claude-1".to_owned())));
    let key = specular_interact::state_key(app.app(), "agents").unwrap_or_default();
    assert_eq!(key["following"], "claude-1");
    assert_eq!(key["agents"][1]["color"], "#8b5cf6");
    app.assert_undo_returns_to_start();
}

#[test]
fn following_keeps_the_cursor_in_view_until_the_person_moves_the_camera() {
    let mut app = app();
    app.with_motion();
    let far = Command::SetRect {
        id: "s2".into(),
        rect: Rect::new(3000.0, 2000.0, 200.0, 200.0),
    };
    call(&mut app, Some("claude-1"), apply(far));
    app.act(Action::Follow(Some("claude-1".to_owned())));
    for step in 1..=200 {
        app.tick(START + step * 16);
    }
    let camera = app.session().camera;
    let tip = camera.world_to_screen(glam::Vec2::new(3024.0, 2024.0));
    assert!(
        (tip - glam::Vec2::new(500.0, 400.0)).length() < 1.0,
        "centred, at {tip}"
    );
    assert_eq!(
        app.app().following().map(|badge| badge.id),
        Some("claude-1".to_owned())
    );
    // Escape lets go, and so does the person's own scroll.
    app.key(Key::Escape);
    assert_eq!(app.app().following(), None);
    app.act(Action::Follow(Some("claude-1".to_owned())));
    app.send(Event::Wheel(WheelInput {
        delta: glam::Vec2::new(0.0, 40.0),
        modifiers: Modifiers::default(),
    }));
    assert_eq!(app.app().following(), None);
    app.assert_undo_returns_to_start();
}
