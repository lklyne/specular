//! The camera transition between the canvas, Device and Fill.
//!
//! Each test names the change that breaks it:
//! - landing: stepping the spring from anywhere but the tick leaves the
//!   camera short of the fitted one, and not ending the transition at the
//!   target owes frames for ever.
//! - centred: starting the camera anywhere but where the new layout rect has
//!   the old rect's screen centre and width makes the item jump at the first
//!   frame.
//! - interrupting: not ending a leave on camera input pulls the canvas back
//!   from where the user put it.
//! - turning: animating the stored rect instead of what is drawn leaves the
//!   page's host a frame behind, starting the shell anywhere but a quarter
//!   turn back makes it jump, and a turn that outlives an undo draws a
//!   shell the page no longer has.

use glam::Vec2;
use specular_core::Camera;
use specular_doc::Rect;
use specular_interact::{Action, Lens, Orientation, Property, Showing};
use specular_testkit::{TestApp, page};

const START: u64 = 1_000_000;

fn show(id: &str) -> Action {
    Action::Show(Showing::Item(id.into()))
}

fn app(motion: bool) -> TestApp {
    let mut app = TestApp::with_entities([page("p1", Rect::new(100.0, 100.0, 400.0, 300.0))]);
    app.viewport((1000.0, 800.0)).zoom(0.5).tick(START);
    if motion {
        app.with_motion();
    }
    app
}

/// The screen centre and width of `p1`'s layout rect.
fn on_screen(app: &TestApp) -> (Vec2, f32) {
    let rect = app.app().page_placement(&"p1".into()).map(|it| it.rect);
    let rect = rect.unwrap_or_default();
    let camera = app.session().camera;
    let centre = camera.world_to_screen(Vec2::new(
        (rect.x + rect.width / 2.0) as f32,
        (rect.y + rect.height / 2.0) as f32,
    ));
    (centre, rect.width as f32 * camera.zoom)
}

fn near(a: (Vec2, f32), b: (Vec2, f32)) -> bool {
    (a.0 - b.0).length() < 0.05 && (a.1 - b.1).abs() < 0.05
}

/// Runs the clock through a whole transition and returns the camera it
/// lands on.
fn finish(app: &mut TestApp, from: u64) -> Camera {
    app.tick(from).tick(from + 2_000);
    app.session().camera
}

fn prepared(app: &mut TestApp, steps: &[Action]) {
    for step in steps {
        app.act(step.clone());
    }
    finish(app, START + 10_000);
}

#[test]
fn a_change_of_view_moves_the_camera_by_a_spring_from_where_the_item_was() {
    let device = Action::SetLens(Lens::Device);
    let rows: [(&str, Vec<Action>, Action); 3] = [
        ("enter", vec![], show("p1")),
        ("lens", vec![show("p1")], device.clone()),
        (
            "leave",
            vec![show("p1"), device],
            Action::Show(Showing::Canvas),
        ),
    ];
    for (name, steps, change) in rows {
        let mut twin = app(false);
        prepared(&mut twin, &steps);
        twin.act(change.clone());
        let fitted = twin.session().camera;

        let mut app = app(true);
        prepared(&mut app, &steps);
        let before = on_screen(&app);
        app.act(change);
        assert_ne!(app.session().camera, fitted, "{name}: lands at once");
        assert!(
            near(on_screen(&app), before),
            "{name}: jumps at the first frame"
        );
        assert!(app.app().motion_running(), "{name}");

        let start = START + 20_000;
        app.tick(start);
        assert!(
            near(on_screen(&app), before),
            "{name}: the clock starts at the first tick"
        );
        app.tick(start + 60);
        assert_ne!(app.session().camera, fitted, "{name}: mid-flight");
        assert!(app.app().motion_running(), "{name}");

        assert_eq!(
            finish(&mut app, start + 100),
            fitted,
            "{name}: lands off target"
        );
        assert!(!app.app().motion_running(), "{name}: still owes frames");
    }
}

#[test]
fn camera_input_ends_a_leave_where_the_camera_is() {
    let mut app = app(true);
    prepared(&mut app, &[show("p1")]);
    app.act(Action::Show(Showing::Canvas));
    let start = START + 20_000;
    app.tick(start).tick(start + 60);
    let mid = app.session().camera;
    assert_ne!(mid, app.app().canvas_camera());

    app.wheel((0.0, 40.0));
    assert!(!app.app().motion_running());
    assert_eq!(app.session().camera.zoom, mid.zoom);
    assert_eq!(app.session().camera.pan, mid.pan + Vec2::new(0.0, 40.0));
    let after = app.session().camera;
    assert_eq!(finish(&mut app, start + 100), after);
}

#[test]
fn rotating_a_page_turns_its_shell_by_a_spring_and_its_rect_at_once() {
    let quarter = std::f32::consts::FRAC_PI_2;
    let lie_down = Action::SetProperty(Property::Orientation(Orientation::Landscape));
    let turning = |app: &TestApp| app.app().turning(&"p1".into());

    let mut still = app(false);
    still.select(&["p1"]).act(lie_down.clone());
    assert_eq!(turning(&still), None, "no transitions, no turn");
    let landed = still.rect("p1");

    let mut app = app(true);
    app.select(&["p1"]).act(lie_down);
    assert_eq!(app.rect("p1"), landed, "the rect waits for the turn");
    let start = turning(&app).map(|it| (it.angle, it.was));
    let was = Rect::new(100.0, 100.0, 400.0, 300.0);
    assert_eq!(start, Some((-quarter, was)), "starts off where it was");
    assert!(app.app().turn_running());

    let at = START + 20_000;
    app.tick(at);
    assert_eq!(turning(&app).map(|it| it.angle), Some(-quarter));
    app.tick(at + 60);
    let mid = turning(&app).map_or(0.0, |it| it.angle);
    assert!(-quarter < mid && mid < 0.0, "mid-flight at {mid}");

    // Turning back on the way carries on from the angle reached.
    app.act(Action::SetProperty(Property::Orientation(
        Orientation::Portrait,
    )));
    let back = turning(&app).map_or(0.0, |it| it.angle);
    assert!((back - (mid + quarter)).abs() < 1e-4, "jumps to {back}");
    app.undo();
    assert_eq!(turning(&app), None, "an undo is not a turn");

    app.tick(at + 2_000);
    assert!(!app.app().turn_running(), "still owes frames");
    app.undo().assert_undo_returns_to_start();
}
