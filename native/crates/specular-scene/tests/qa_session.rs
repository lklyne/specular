//! Defects found by using the features together (the end-to-end QA pass),
//! each kept as the test that failed before its fix.

use specular_doc::Rect;
use specular_testkit::{ALT, TestApp, shape};

#[test]
fn an_option_drag_draws_each_copy_where_it_will_land() {
    let mut app = TestApp::with_entities([shape("s", Rect::new(100.0, 100.0, 100.0, 100.0))]);
    app.hold(ALT).press((150.0, 150.0)).drag_to((450.0, 150.0));
    let preview = app.app().copy_preview();
    assert_eq!(
        preview.map(|preview| (preview.entities, preview.delta.x, preview.delta.y)),
        Some((vec!["s".into()], 300.0, 0.0))
    );
    let scene = app.scene_snapshot();
    let original = (scene.lines())
        .find(|line| line.starts_with("canvas rect 100,100 100x100"))
        .unwrap_or_default();
    // The ghost is the shape itself, drawn again where the copy will land.
    let ghost = format!("{} opacity=0.5", original.replace("100,100", "400,100"));
    assert!(
        !original.is_empty() && scene.contains(&ghost),
        "no {ghost:?} in:\n{scene}"
    );
    // And the outline it will have, one pixel outside the rect.
    assert!(
        scene.contains("screen rect 399,99 102x102 stroke=#3b82f6"),
        "no outline at the copy:\n{scene}"
    );
}

#[test]
fn a_zoomed_out_copy_of_a_group_and_a_page_is_drawn_without_a_page_border_or_title() {
    let mut app = TestApp::with_entities([
        specular_testkit::group("g", Rect::new(100.0, 100.0, 200.0, 100.0)),
        specular_testkit::page("p", Rect::new(100.0, 400.0, 200.0, 100.0)),
    ]);
    app.zoom(0.5);
    let at = |app: &TestApp, x: f64, y: f64| {
        let screen = app
            .app()
            .session()
            .camera
            .world_to_screen((x as f32, y as f32).into());
        (screen.x, screen.y)
    };
    let (from, to) = (at(&app, 150.0, 150.0), at(&app, 450.0, 150.0));
    app.select(&["g", "p"]);
    app.hold(ALT).press(from).drag_to(to);
    let scene = app.scene_snapshot();
    // The group's border, half a zoom from its original, at half strength.
    assert!(
        scene.contains("screen rect 200,50 100x50 r=2 stroke=#71717a40/1.5/inside opacity=0.5"),
        "{scene}"
    );
    // The page is drawn again, but its border and title are not.
    assert!(
        scene.contains("canvas page p 400,400 200x100 r=8 opacity=0.5"),
        "{scene}"
    );
    assert!(!scene.contains("screen rect 200,200 100x50"), "{scene}");
    assert_eq!(scene.matches("example.com/p").count(), 1, "{scene}");
}
