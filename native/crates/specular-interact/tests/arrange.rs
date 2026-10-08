//! Arrange in a row, a column or a grid, and focus: what the popup's buttons
//! send. Each arrange is one undo step that keeps the footprint the items
//! have and evens the spacing inside it.

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
    assert_eq!(app.rect("a").x, app.rect("c").x, "a grid keeps its shape");
    assert_eq!(app.rect("a").y, app.rect("b").y);
    assert_ne!(app.rect("b").x, app.rect("a").x);
    app.assert_undo_returns_to_start();
}

#[test]
fn a_group_takes_what_is_inside_it_along() {
    let mut app = TestApp::with_entities([
        group("g", Rect::new(0.0, 0.0, 200.0, 100.0)),
        inside("g", shape("m", Rect::new(20.0, 20.0, 60.0, 60.0))),
        shape("s", Rect::new(700.0, 300.0, 100.0, 100.0)),
    ]);
    app.select(&["g", "s"])
        .act(Action::Arrange(ArrangeMode::Row));
    let moved = app.rect("g").y;
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
    let mut app = three();
    app.select(&["a"]).act(Action::Arrange(ArrangeMode::Row));
    assert!(!app.app().can_undo());

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
    let camera = app.session().camera;
    assert!(camera.zoom > 0.99, "a small item is framed at full size");
    let on_screen = camera.world_to_screen(glam::Vec2::new(450.0, 60.0));
    assert!((on_screen.x - 500.0).abs() < 1.0, "centred across");
    assert!(!app.app().can_undo());
}
