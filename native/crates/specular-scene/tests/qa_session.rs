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
