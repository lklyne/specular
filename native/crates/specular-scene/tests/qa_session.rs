//! Defects found by using the features together (the end-to-end QA pass),
//! each kept as the test that failed before its fix.

use specular_doc::Rect;
use specular_testkit::{ALT, TestApp, shape};

#[test]
fn an_option_drag_outlines_where_the_copy_will_land() {
    let mut app = TestApp::with_entities([shape("s", Rect::new(100.0, 100.0, 100.0, 100.0))]);
    app.hold(ALT).press((150.0, 150.0)).drag_to((450.0, 150.0));
    assert_eq!(
        app.app().copy_preview(),
        [Rect::new(400.0, 100.0, 100.0, 100.0)]
    );
    let scene = app.scene_snapshot();
    // The ghost sits one pixel outside the rect, as a selection's outline does.
    assert!(
        scene.contains("screen rect 399,99 102x102 fill=#3b82f6"),
        "no outline at the copy:\n{scene}"
    );
}
