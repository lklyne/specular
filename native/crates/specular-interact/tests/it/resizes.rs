//! Resize math, table by table: what each handle moves, where each kind
//! stops, and how far a selection of several can shrink. The gestures
//! around it (cursors, ratios, text, pages, carried items) are scenarios in
//! `fixtures/scenarios/resize.txt`.

use specular_doc::{Entity, Rect};
use specular_testkit::{TestApp, drawing, file, group, page, shape, text};

const S: Rect = Rect::new(100.0, 100.0, 200.0, 100.0);
/// The handles of [`S`]: corners clockwise from the top left, then the
/// middles of the sides clockwise from the top.
const TOP_LEFT: (f32, f32) = (100.0, 100.0);
const TOP_RIGHT: (f32, f32) = (300.0, 100.0);
const BOTTOM_RIGHT: (f32, f32) = (300.0, 200.0);
const BOTTOM_LEFT: (f32, f32) = (100.0, 200.0);
const TOP: (f32, f32) = (200.0, 100.0);
const RIGHT: (f32, f32) = (300.0, 150.0);
const BOTTOM: (f32, f32) = (200.0, 200.0);
const LEFT: (f32, f32) = (100.0, 150.0);

/// `entity` alone on the canvas, selected, so its handles exist.
fn selected(entity: Entity) -> TestApp {
    let id = entity.id.as_str().to_owned();
    let mut app = TestApp::with_entities([entity]);
    app.select(&[id.as_str()]);
    app
}

#[test]
fn each_handle_moves_its_own_edges_and_holds_the_others() {
    // Every handle is dragged 40 right and 20 down.
    let cases = [
        (TOP_LEFT, Rect::new(140.0, 120.0, 160.0, 80.0)),
        (TOP_RIGHT, Rect::new(100.0, 120.0, 240.0, 80.0)),
        (BOTTOM_RIGHT, Rect::new(100.0, 100.0, 240.0, 120.0)),
        (BOTTOM_LEFT, Rect::new(140.0, 100.0, 160.0, 120.0)),
        (TOP, Rect::new(100.0, 120.0, 200.0, 80.0)),
        (RIGHT, Rect::new(100.0, 100.0, 240.0, 100.0)),
        (BOTTOM, Rect::new(100.0, 100.0, 200.0, 120.0)),
        (LEFT, Rect::new(140.0, 100.0, 160.0, 100.0)),
    ];
    for (handle, expected) in cases {
        let mut app = selected(shape("s", S));
        app.drag(handle, (handle.0 + 40.0, handle.1 + 20.0));
        assert_eq!(app.rect("s"), expected, "from the handle at {handle:?}");
        app.assert_undo_returns_to_start();
    }
}

#[test]
fn every_kind_stops_at_its_own_minimum_size() {
    let square = Rect::new(100.0, 100.0, 400.0, 400.0);
    let cases = [
        (page("e", square), (320.0, 200.0)),
        // A note is at least square.
        (text("e", square), (100.0, 100.0)),
        (file("e", square), (80.0, 80.0)),
        (group("e", square), (120.0, 80.0)),
        // 16 is the limit, and the grid line past it is the first place the
        // edge can stop.
        (drawing("e", square), (20.0, 20.0)),
        (shape("e", square), (24.0, 24.0)),
    ];
    for (entity, minimum) in cases {
        let kind = entity.kind.name();
        let mut app = selected(entity);
        app.drag((500.0, 500.0), (0.0, 0.0));
        let rect = app.rect("e");
        assert_eq!((rect.width, rect.height), minimum, "for a {kind}");
        assert_eq!((rect.x, rect.y), (100.0, 100.0), "for a {kind}");
        app.assert_undo_returns_to_start();
    }
}

#[test]
fn the_bounds_stop_at_twenty_units_and_nothing_inside_vanishes() {
    // A speck between the two shapes rounds to nothing at this scale.
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(300.0, 100.0, 100.0, 100.0)),
        shape("c", Rect::new(220.0, 100.0, 5.0, 5.0)),
    ]);
    app.select(&["a", "b", "c"]);
    app.drag((400.0, 200.0), (-900.0, -900.0));
    assert_eq!(
        (app.rect("a"), app.rect("b"), app.rect("c")),
        (
            Rect::new(100.0, 100.0, 7.0, 20.0),
            Rect::new(113.0, 100.0, 7.0, 20.0),
            Rect::new(108.0, 100.0, 1.0, 1.0)
        )
    );
    app.assert_undo_returns_to_start();
}
