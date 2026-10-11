//! A flick on a touch page: the page's finger glides on after the user's
//! has left, and is lifted once it has come to rest.

use specular_core::{InputEvent, PointerEventKind, TouchPhase};
use specular_interact::{Effect, PageNotice};
use specular_testkit::TestApp;

/// A swipe up of 200 pixels in 40 ms: 5000 pixels a second to the end.
const FAST: [f32; 5] = [40.0; 5];
/// A swipe up that slows to a stop.
const SLOWING: [f32; 6] = [80.0, 60.0, 30.0, 10.0, 3.0, 1.0];
/// How far a flick at 5000 pixels a second glides: the speed over the decay
/// rate, short of the last crawl that is dropped.
const GLIDE: f32 = -2490.0;

/// One page, 400 by 800 at (100, 100), on `device`, entered.
fn app_on(device: &str) -> TestApp {
    let mut app = TestApp::from_canvas(&format!(
        r#"{{"nodes":[{{"id":"p1","type":"link","x":100,"y":100,"width":400,"height":800,
            "url":"https://example.com/p1","metadata":{{"deviceId":"{device}"}}}}],"edges":[]}}"#
    ));
    app.click((300.0, 500.0)).click((300.0, 500.0));
    app
}

/// Presses at the page's y of 700 and moves up by each of `steps`, one a
/// frame. Returns the time of the last move and the page's y it ended at.
fn swipe_up(app: &mut TestApp, steps: &[f32]) -> (u64, f32) {
    let (mut now, mut y) = (1000, 800.0);
    app.tick(now).press((300.0, y));
    for step in steps {
        now += 8;
        y -= step;
        app.tick(now).drag_to((300.0, y));
    }
    app.take_effects();
    (now, y - 100.0)
}

/// Runs the clock for `ms` from `from`, a frame at a time.
fn run(app: &mut TestApp, from: u64, ms: u64) {
    for now in (from + 8..=from + ms).step_by(8) {
        app.tick(now);
    }
}

/// What the page was told since the last call: the y its finger was last
/// moved to, and what else it heard of a finger or a mouse button.
fn told(app: &mut TestApp) -> (Option<f32>, Vec<&'static str>) {
    let (mut y, mut heard) = (None, Vec::new());
    for effect in app.take_effects() {
        let Effect::ForwardInput { event, .. } = effect else {
            continue;
        };
        match event {
            InputEvent::Touch(touch) => match touch.phase {
                TouchPhase::Move => y = Some(touch.position.y),
                TouchPhase::Start => heard.push("finger down"),
                TouchPhase::End => heard.push("finger up"),
            },
            InputEvent::Pointer(pointer) => match pointer.kind {
                PointerEventKind::Down { .. } => heard.push("button down"),
                PointerEventKind::Up { .. } => heard.push("button up"),
                PointerEventKind::Move | PointerEventKind::Leave => {}
            },
            InputEvent::Wheel(_)
            | InputEvent::Key(_)
            | InputEvent::Ime(_)
            | InputEvent::Edit(_) => {}
        }
    }
    (y, heard)
}

#[test]
fn a_flick_on_a_touch_page_glides_and_nothing_else_does() {
    let phone = "iphone-14-pro";
    // The swipe, how long after its last move the button comes up, and how
    // far the page's finger goes on by itself.
    let rows = [
        (
            "released in mid swipe",
            phone,
            &FAST[..],
            0,
            GLIDE,
            "finger up",
        ),
        // A tap-and-drag keeps the button down after the finger has left:
        // the glide starts when the pointer stops dead, not then.
        ("the button late", phone, &FAST[..], 400, GLIDE, "finger up"),
        (
            "slowed to a stop",
            phone,
            &SLOWING[..],
            400,
            0.0,
            "finger up",
        ),
        // A page that is no touch device hears of a mouse, and never glides.
        ("on a laptop", "laptop", &FAST[..], 0, 0.0, "button up"),
    ];
    for (name, device, steps, late_ms, want, lift) in rows {
        let mut app = app_on(device);
        let (stopped, from) = swipe_up(&mut app, steps);
        run(&mut app, stopped, late_ms);
        app.release();
        run(&mut app, stopped + late_ms, 5000);
        let (y, heard) = told(&mut app);
        let glided = y.map_or(0.0, |y| y - from);
        assert!((glided - want).abs() < 15.0, "{name}: glided {glided}");
        // The finger is lifted once, at the end of the glide.
        assert_eq!(heard, [lift], "{name}");
        assert!(!app.app().flick_running(), "{name}: still running");
    }
}

#[test]
fn a_press_or_the_finger_coming_back_catches_a_gliding_page() {
    // A tap stops the page: its finger is already down, so the page hears
    // of no press, and of the release once the finger has rested.
    let mut app = app_on("iphone-14-pro");
    let (stopped, from) = swipe_up(&mut app, &FAST);
    app.release();
    run(&mut app, stopped, 200);
    let (y, buttons) = told(&mut app);
    let caught_at = y.unwrap_or(from);
    assert!(caught_at - from < -500.0, "glided to {caught_at}");
    assert_eq!(buttons, [""; 0]);
    app.press((300.0, 500.0)).release();
    run(&mut app, stopped + 200, 1000);
    assert_eq!(told(&mut app), (None, vec!["finger up"]));
    assert!(!app.app().flick_running());

    // The finger back on the trackpad while a tap-and-drag still holds the
    // button: the page goes on from where it was caught.
    let mut app = app_on("iphone-14-pro");
    let (stopped, from) = swipe_up(&mut app, &FAST);
    run(&mut app, stopped, 200);
    let caught_at = told(&mut app).0.unwrap_or(from);
    assert!(caught_at - from < -500.0, "glided to {caught_at}");
    app.drag_to((300.0, 600.0)).drag_to((300.0, 630.0));
    run(&mut app, stopped + 200, 16);
    let (y, buttons) = told(&mut app);
    assert_eq!(buttons, [""; 0]);
    let went = y.map_or(0.0, |y| y - caught_at);
    assert!((went - 30.0).abs() < 0.01, "went on by {went}");
}

#[test]
fn a_glide_that_runs_into_the_end_of_the_page_lifts_the_finger_there() {
    // The page scrolls under the glide for 200 ms and then says no more:
    // it is at its end, where a finger that pushed on would stretch it.
    let mut app = app_on("iphone-14-pro");
    let (stopped, from) = swipe_up(&mut app, &FAST);
    app.release();
    for now in (stopped + 8..=stopped + 200).step_by(8) {
        let y = (now - stopped) as f64;
        app.tick(now)
            .page_reports("p1", PageNotice::Scrolled { x: 0.0, y });
    }
    run(&mut app, stopped + 200, 400);
    let (y, heard) = told(&mut app);
    let glided = y.map_or(0.0, |y| y - from);
    // 300 ms at 5000 pixels a second, slowing: well short of the whole glide.
    assert!((-1300.0..-1000.0).contains(&glided), "glided {glided}");
    assert_eq!(heard, ["finger up"]);
    assert!(!app.app().flick_running());
}

#[test]
fn a_flick_with_nothing_to_scroll_is_stopped_by_the_pages_answer() {
    // The page never scrolled under the swipe, so it is asked if the glide
    // has anything to scroll. With room it glides to its end.
    for (room, glides) in [(true, true), (false, false)] {
        let mut app = app_on("iphone-14-pro");
        let (stopped, from) = swipe_up(&mut app, &FAST);
        app.release();
        run(&mut app, stopped, 16);
        let asked = (app.take_effects().into_iter())
            .filter(
                |effect| matches!(effect, Effect::AskScrollRoom { toward, .. } if toward.y < 0.0),
            )
            .count();
        assert_eq!(asked, 1, "room {room}");
        app.page_reports("p1", PageNotice::ScrollRoom(room));
        run(&mut app, stopped + 16, 5000);
        let (y, heard) = told(&mut app);
        let glided = y.is_some_and(|y| y - from < -2000.0);
        assert_eq!((glided, heard), (glides, vec!["finger up"]), "room {room}");
    }
}
