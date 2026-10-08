//! Scene snapshots for the guides a move shows: an alignment line in the
//! selection colour, and the pink capped measures of an even run.

use specular_doc::Rect;
use specular_testkit::{TestApp, assert_scene_snapshot, shape};

#[test]
fn a_drag_level_with_two_neighbours_draws_lines_and_gap_measures() {
    let mut app = TestApp::with_entities([
        shape("left", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("right", Rect::new(500.0, 100.0, 100.0, 100.0)),
        shape("a", Rect::new(300.0, 400.0, 100.0, 100.0)),
    ]);
    app.viewport((1600.0, 1000.0));
    // At half zoom the lines are still a pixel wide and the caps 18 tall.
    app.zoom(0.5);
    let middle = app
        .app()
        .session()
        .camera
        .world_to_screen((350.0, 450.0).into());
    let level = app
        .app()
        .session()
        .camera
        .world_to_screen((350.0, 150.0).into());
    app.press(middle).drag_to(level);
    assert_scene_snapshot!(app);
}
