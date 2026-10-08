//! Arrange in a row, a column or a grid, and focus: what the popup's buttons
//! send. Each arrange is one undo step that keeps the footprint the items
//! have and evens the spacing inside it.

use specular_core::Camera;
use specular_doc::Rect;
use specular_interact::{Action, ArrangeMode};
use specular_testkit::{TestApp, group, inside, shape};

fn three() -> TestApp {
    TestApp::with_entities([
        shape("a", Rect::new(0.0, 0.0, 100.0, 80.0)),
        shape("b", Rect::new(150.0, 40.0, 100.0, 80.0)),
        shape("c", Rect::new(400.0, 20.0, 100.0, 80.0)),
    ])
}

#[test]
fn a_row_a_column_and_a_grid_even_the_spacing_as_one_step() {
    let mut app = three();
    app.select(&["a", "b", "c"])
        .act(Action::Arrange(ArrangeMode::Row));
    assert_eq!(app.rect("a"), Rect::new(0.0, 0.0, 100.0, 80.0));
    assert_eq!(app.rect("b"), Rect::new(200.0, 0.0, 100.0, 80.0));
    assert_eq!(app.rect("c"), Rect::new(400.0, 0.0, 100.0, 80.0));

    app.undo();
    assert_eq!(app.rect("b"), Rect::new(150.0, 40.0, 100.0, 80.0));
    assert_eq!(app.rect("c"), Rect::new(400.0, 20.0, 100.0, 80.0));
    app.redo();
    assert_eq!(app.rect("b"), Rect::new(200.0, 0.0, 100.0, 80.0));
    app.assert_undo_returns_to_start();

    let mut app = TestApp::with_entities([
        shape("a", Rect::new(10.0, 0.0, 80.0, 50.0)),
        shape("b", Rect::new(40.0, 100.0, 80.0, 50.0)),
        shape("c", Rect::new(0.0, 500.0, 80.0, 50.0)),
    ]);
    app.select(&["a", "b", "c"])
        .act(Action::Arrange(ArrangeMode::Column));
    let lefts = ["a", "b", "c"].map(|id| app.rect(id).x);
    assert_eq!(lefts, [0.0; 3], "a column lines the left edges up");
    let (a, b, c) = (app.rect("a"), app.rect("b"), app.rect("c"));
    assert_eq!(b.y - (a.y + a.height), c.y - (b.y + b.height));
    app.assert_undo_returns_to_start();

    let mut app = TestApp::with_entities([
        shape("a", Rect::new(0.0, 0.0, 100.0, 100.0)),
        shape("b", Rect::new(130.0, 8.0, 100.0, 100.0)),
        shape("c", Rect::new(12.0, 220.0, 100.0, 100.0)),
    ]);
    app.select(&["a", "b", "c"])
        .act(Action::Arrange(ArrangeMode::Grid));
    // Two columns and two rows, the gaps floored at 80 and on the grid.
    assert_eq!(app.rect("a"), Rect::new(0.0, 0.0, 100.0, 100.0));
    assert_eq!(app.rect("b"), Rect::new(200.0, 0.0, 100.0, 100.0));
    assert_eq!(app.rect("c"), Rect::new(0.0, 220.0, 100.0, 100.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_cluster_collapsed_on_the_axis_spreads_over_its_other_extent_on_the_grid() {
    // Stacked above one another, arranged as a row: the 500 they span down
    // is the extent, so the gap is 100.
    let stacked = |last: f64| {
        TestApp::with_entities([
            shape("a", Rect::new(0.0, 0.0, 100.0, 80.0)),
            shape("b", Rect::new(0.0, 200.0, 100.0, 80.0)),
            shape("c", Rect::new(0.0, last, 100.0, 80.0)),
        ])
    };
    let lefts = |app: &TestApp| ["a", "b", "c"].map(|id| app.rect(id).x);
    let mut app = stacked(420.0);
    app.select(&["a", "b", "c"])
        .act(Action::Arrange(ArrangeMode::Row));
    assert_eq!(lefts(&app), [0.0, 200.0, 400.0]);
    app.assert_undo_returns_to_start();
    // A gap of 85 is put on the grid, at 80.
    let mut app = stacked(390.0);
    app.select(&["a", "b", "c"])
        .act(Action::Arrange(ArrangeMode::Row));
    assert_eq!(lefts(&app), [0.0, 180.0, 360.0]);
}

#[test]
fn a_grid_snaps_a_line_of_items_and_keeps_items_inside_a_wide_one_in_its_band() {
    // `big` spans the others on x, so they are one column; down the page
    // they are three rows, 80 apart at least and on the grid.
    let mut app = TestApp::with_entities([
        shape("big", Rect::new(0.0, 0.0, 300.0, 50.0)),
        shape("in", Rect::new(50.0, 100.0, 50.0, 50.0)),
        shape("out", Rect::new(200.0, 200.0, 50.0, 50.0)),
    ]);
    app.select(&["big", "in", "out"])
        .act(Action::Arrange(ArrangeMode::Grid));
    assert_eq!(app.rect("big"), Rect::new(0.0, 0.0, 300.0, 50.0));
    assert_eq!(app.rect("in"), Rect::new(60.0, 140.0, 50.0, 50.0));
    assert_eq!(app.rect("out"), Rect::new(200.0, 280.0, 50.0, 50.0));
    app.assert_undo_returns_to_start();
}

#[test]
fn a_group_takes_what_is_inside_it_along() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(0.0, 300.0, 200.0, 100.0)),
        inside("g", shape("m", Rect::new(20.0, 320.0, 60.0, 60.0))),
        shape("s", Rect::new(700.0, 0.0, 100.0, 100.0)),
    ]);
    app.select(&["g", "s"])
        .act(Action::Arrange(ArrangeMode::Row));
    let moved = app.rect("g").y;
    assert_eq!(moved, 0.0, "the group moves up to the top of the row");
    assert_eq!(app.rect("s").y, moved, "tops line up");
    assert_eq!(
        app.rect("m"),
        Rect::new(20.0, 20.0 + moved, 60.0, 60.0),
        "the member moves with its group"
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn one_item_has_nothing_to_arrange_and_a_tidy_row_records_no_step() {
    // Off the grid, so that arranging it alone would show.
    let mut app = TestApp::with_entities([shape("a", Rect::new(7.0, 3.0, 100.0, 80.0))]);
    app.select(&["a"]).act(Action::Arrange(ArrangeMode::Row));
    assert_eq!(app.rect("a"), Rect::new(7.0, 3.0, 100.0, 80.0));
    assert!(!app.app().can_undo());

    let mut app = three();

    app.select(&["a", "b", "c"])
        .act(Action::Arrange(ArrangeMode::Row))
        .act(Action::Arrange(ArrangeMode::Row));
    app.undo();
    assert_eq!(
        app.rect("b"),
        Rect::new(150.0, 40.0, 100.0, 80.0),
        "the second arrange changed nothing, so one undo undoes the first"
    );
    app.assert_undo_returns_to_start();
}

#[test]
fn focus_frames_the_selection_and_changes_nothing_in_the_document() {
    let mut app = three();
    app.viewport((1000.0_f32, 800.0_f32));
    app.select(&["c"]).act(Action::FocusSelection);
    // A small item is framed at full size, centred both ways: its middle,
    // (450, 60), is the middle of the viewport.
    assert_eq!(
        app.session().camera,
        Camera::new(glam::Vec2::new(50.0, 340.0), 1.0)
    );
    assert!(!app.app().can_undo());

    // A wide one is fitted by its wide side, with 64 around it.
    let mut app = TestApp::with_entities([shape("wide", Rect::new(0.0, 0.0, 2000.0, 100.0))]);
    app.viewport((1000.0_f32, 800.0_f32));
    app.select(&["wide"]).act(Action::FocusSelection);
    let camera = app.session().camera;
    let want = Camera::new(glam::Vec2::new(64.0, 400.0 - 50.0 * 0.436), 0.436);
    assert!(
        (camera.pan - want.pan).abs().max_element() < 0.01
            && (camera.zoom - want.zoom).abs() < 1e-4,
        "{camera:?}"
    );

    // The toolbar and the sidebar leave less of the viewport free.
    let mut free = TestApp::with_entities([shape("a", Rect::new(0.0, 0.0, 100.0, 100.0))]);
    free.select(&["a"]).act(Action::FocusSelection);
    free.with_panels().act(Action::FocusSelection);
    let middle = free
        .session()
        .camera
        .world_to_screen(glam::Vec2::new(50.0, 50.0));
    let viewport = free.session().viewport;
    assert!(
        middle.y > viewport.y / 2.0,
        "{middle} should be centred below the toolbar"
    );
}
