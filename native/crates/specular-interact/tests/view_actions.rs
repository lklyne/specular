//! Select all and the View menu's zoom commands.

use glam::Vec2;
use specular_doc::Rect;
use specular_interact::{Action, Key};
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
fn select_all_waits_for_a_drag_to_end() {
    let mut app = TestApp::with_pages(2);
    app.select(&["p1"])
        .press((200.0, 150.0))
        .drag_to((300.0, 250.0));
    app.act(Action::SelectAll);
    assert_eq!(app.selected_ids(), ["p1"]);
    app.release();
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
fn zoom_stops_at_the_cameras_limits() {
    let mut app = TestApp::with_pages(1);
    app.viewport(VIEWPORT);
    for _ in 0..40 {
        app.act(Action::ZoomIn);
    }
    assert!((app.session().camera.zoom - 3.0).abs() < 1e-6);
    for _ in 0..80 {
        app.act(Action::ZoomOut);
    }
    assert!((app.session().camera.zoom - 0.02).abs() < 1e-6);
}

#[test]
fn zoom_to_100_keeps_the_middle_of_the_viewport() {
    let mut app = TestApp::with_pages(1);
    app.viewport(VIEWPORT).zoom(0.4);
    let centre = app.session().camera.screen_to_world(VIEWPORT / 2.0);
    app.chord(CMD, Key::Char('0'));
    let camera = app.session().camera;
    assert!((camera.zoom - 1.0).abs() < f32::EPSILON);
    assert!(
        camera
            .screen_to_world(VIEWPORT / 2.0)
            .abs_diff_eq(centre, 1e-2)
    );
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
fn zoom_to_fit_works_from_inside_a_page() {
    let mut app = TestApp::with_pages(3);
    app.viewport(VIEWPORT).double_click((200.0, 150.0));
    app.take_effects();
    app.chord(CMD, Key::Char('1'));
    assert!((app.session().camera.zoom - 0.545).abs() < 1e-6);
    // The page did not get the key.
    assert_eq!(app.take_effects(), []);
}

#[test]
fn zoom_to_fit_on_an_empty_canvas_goes_home() {
    let mut app = TestApp::empty();
    app.viewport(VIEWPORT).zoom(0.3);
    app.act(Action::ZoomToFit);
    assert_eq!(app.session().camera, specular_core::Camera::default());
}

#[test]
fn zoom_changes_nothing_in_the_document() {
    let mut app = TestApp::with_pages(1);
    app.viewport(VIEWPORT).take_effects();
    app.act(Action::ZoomIn).act(Action::ZoomToFit);
    assert_eq!(app.take_effects(), []);
    assert!(!app.app().can_undo());
}
