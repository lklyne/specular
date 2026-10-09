//! Select all and the View menu's zoom commands.

use glam::Vec2;
use specular_doc::Rect;
use specular_interact::Key;
use specular_interact::panel::builtin::CHROME_HEIGHT;
use specular_testkit::{CMD, TestApp, group, inside, page, shape};

const VIEWPORT: Vec2 = Vec2::new(1000.0, 800.0);

#[test]
fn select_all_takes_everything_that_is_not_inside_a_group() {
    let mut app = TestApp::with_entities([
        page("p1", Rect::new(0.0, 0.0, 400.0, 300.0)),
        group("g", Rect::new(500.0, 0.0, 300.0, 300.0)),
        inside("g", shape("a", Rect::new(520.0, 20.0, 100.0, 100.0))),
        shape("b", Rect::new(0.0, 400.0, 100.0, 100.0)),
    ]);
    app.chord(CMD, Key::Char('a'));
    assert_eq!(app.selected_ids(), ["p1", "g", "b"]);
    // The group brings its member along (ADR 0034).
    assert!(app.app().selection_scope().holds(&"a".into()));
}

#[test]
fn zoom_in_and_out_step_about_the_middle_of_the_viewport() {
    let mut app = TestApp::with_pages(1);
    app.viewport(VIEWPORT);
    let centre = app.session().camera.screen_to_world(VIEWPORT / 2.0);
    app.chord(CMD, Key::Char('='));
    let camera = app.session().camera;
    assert!((camera.zoom - 1.25).abs() < 1e-6);
    assert!(
        camera
            .screen_to_world(VIEWPORT / 2.0)
            .abs_diff_eq(centre, 1e-3)
    );
    app.chord(CMD, Key::Char('-')).chord(CMD, Key::Char('-'));
    assert!((app.session().camera.zoom - 0.8).abs() < 1e-6);
}

#[test]
fn zoom_to_fit_centres_everything_with_room_around_it() {
    // p1 at (100, 100) to p3 ending at (1700, 400).
    let mut app = TestApp::with_pages(3);
    app.viewport(VIEWPORT);
    app.chord(CMD, Key::Char('1'));
    let camera = app.session().camera;
    // 1600 wide into 1000 - 2 * 64.
    assert!((camera.zoom - 0.545).abs() < 1e-6);
    let centre = camera.world_to_screen(Vec2::new(900.0, 250.0));
    assert!(centre.abs_diff_eq(VIEWPORT / 2.0, 1e-2));
}

#[test]
fn zoom_to_fit_leaves_the_chrome_clear_when_the_built_in_panels_are_on() {
    // Taller than wide, so the height is what limits the fit.
    let mut app = TestApp::with_entities([shape("s", Rect::new(0.0, 0.0, 200.0, 2000.0))]);
    app.viewport(VIEWPORT).with_panels();
    app.chord(CMD, Key::Char('1'));
    let camera = app.session().camera;
    // 2000 tall into what the three rows leave, less 64 above and below.
    let room = VIEWPORT.y - CHROME_HEIGHT - 2.0 * 64.0;
    assert!((camera.zoom - room / 2000.0).abs() < 1e-6);
    let top = camera.world_to_screen(Vec2::new(100.0, 0.0)).y;
    let bottom = camera.world_to_screen(Vec2::new(100.0, 2000.0)).y;
    // Centred under the chrome, so the same room above and below.
    assert!((top - CHROME_HEIGHT - 64.0).abs() < 1e-3);
    assert!((VIEWPORT.y - bottom - 64.0).abs() < 1e-3);
}

#[test]
fn zoom_to_fit_works_from_inside_a_page() {
    let mut app = TestApp::with_pages(3);
    app.viewport(VIEWPORT).double_click((200.0, 150.0));
    app.take_effects();
    app.chord(CMD, Key::Char('1'));
    assert!((app.session().camera.zoom - 0.545).abs() < 1e-6);
    // The page did not get the key.
    assert_eq!(app.take_effects(), []);
}
