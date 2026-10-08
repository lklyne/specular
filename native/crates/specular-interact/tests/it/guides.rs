//! Alignment and distribution guides during a move and a resize (ADR 0012):
//! they confirm what the grid has already lined up, and never pull.

use specular_doc::Rect;
use specular_interact::{GuideAxis, GuideReference, Guides};
use specular_testkit::{ALT, SHIFT, TestApp, group, inside, shape};

use GuideAxis::{Horizontal, Vertical};
use GuideReference::{Bottom, Left, Right, Top};

const VIEWPORT: (f32, f32) = (1600.0, 1000.0);

/// `a` to drag, with `b` to its right on the same grid.
fn pair() -> TestApp {
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 300.0, 100.0, 100.0)),
        shape("b", Rect::new(400.0, 100.0, 100.0, 100.0)),
    ]);
    app.viewport(VIEWPORT);
    app
}

/// One alignment: the dragged reference, the neighbour's, and where.
type Line = (GuideReference, GuideReference, f64);

/// The alignments along `axis`.
fn lines(guides: &Guides, axis: GuideAxis) -> Vec<Line> {
    (guides.alignment.iter())
        .filter(|guide| guide.axis == axis)
        .map(|guide| {
            (
                guide.dragged_reference,
                guide.candidate_reference,
                guide.coordinate,
            )
        })
        .collect()
}

#[test]
fn a_move_shows_the_alignments_the_grid_leaves_it_on() {
    // (pointer travel from the middle of `a`, the horizontal lines).
    let rows: [((f32, f32), Vec<Line>); 4] = [
        // Level with `b`: both edges, so the centre goes unsaid.
        (
            (0.0, -200.0),
            vec![(Top, Top, 100.0), (Bottom, Bottom, 200.0)],
        ),
        // The grid rounds 7 short back onto the line.
        (
            (0.0, -193.0),
            vec![(Top, Top, 100.0), (Bottom, Bottom, 200.0)],
        ),
        // One row down: `a`'s top on `b`'s middle is not on the grid, its
        // top on `b`'s bottom is.
        ((0.0, -100.0), vec![(Top, Bottom, 200.0)]),
        ((0.0, -160.0), vec![]),
    ];
    for (travel, want) in rows {
        let mut app = pair();
        app.press((150.0, 350.0))
            .drag_to((150.0 + travel.0, 350.0 + travel.1));
        assert_eq!(lines(&app.app().guides(), Horizontal), want, "{travel:?}");
        app.release();
        assert!(app.app().guides().is_empty(), "guides end with the drag");
    }
}

#[test]
fn a_press_that_has_not_travelled_shows_nothing() {
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(400.0, 100.0, 100.0, 100.0)),
    ]);
    app.viewport(VIEWPORT);
    app.press((150.0, 150.0));
    assert!(app.app().guides().is_empty());
}

#[test]
fn shift_holds_an_off_grid_edge_where_a_guide_can_find_it() {
    // `a` sits 5 off the grid, level with `b`. Shift keeps its y exactly,
    // so the tops stay aligned; without Shift the grid takes it off the
    // line and the guide goes.
    let entities = || {
        [
            shape("a", Rect::new(100.0, 105.0, 100.0, 100.0)),
            shape("b", Rect::new(400.0, 105.0, 100.0, 100.0)),
        ]
    };
    let mut app = TestApp::with_entities(entities());
    app.viewport(VIEWPORT);
    app.press((150.0, 155.0))
        .hold(SHIFT)
        .drag_to((210.0, 158.0));
    assert_eq!(
        lines(&app.app().guides(), Horizontal),
        [(Top, Top, 105.0), (Bottom, Bottom, 205.0)]
    );

    let mut app = TestApp::with_entities(entities());
    app.viewport(VIEWPORT);
    app.press((150.0, 155.0)).drag_to((210.0, 158.0));
    assert_eq!(lines(&app.app().guides(), Horizontal), []);
}

#[test]
fn the_neighbours_are_the_ones_in_view_when_the_drag_began() {
    // `far` is level with `a` but off the right of the viewport at the
    // press. Panning it into view mid-drag does not make it a neighbour.
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("far", Rect::new(1700.0, 100.0, 100.0, 100.0)),
    ]);
    app.viewport(VIEWPORT);
    app.press((150.0, 150.0)).drag_to((190.0, 150.0));
    assert!(app.app().guides().is_empty());
    app.wheel((400.0, 0.0));
    assert!(app.app().guides().is_empty());
}

#[test]
fn a_group_in_view_stands_for_its_members() {
    // `m` is level with `a`, but its group is the neighbour: only the
    // group's own edges count, and they are elsewhere.
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 400.0, 100.0, 100.0)),
        group("g", Rect::new(380.0, 60.0, 300.0, 300.0)),
        inside("g", shape("m", Rect::new(400.0, 100.0, 100.0, 100.0))),
    ]);
    app.viewport(VIEWPORT);
    app.press((150.0, 450.0)).drag_to((150.0, 150.0));
    assert_eq!(lines(&app.app().guides(), Horizontal), []);
    app.drag_to((150.0, 110.0));
    assert_eq!(lines(&app.app().guides(), Horizontal), [(Top, Top, 60.0)]);
}

#[test]
fn an_option_drag_lines_its_copy_up_with_the_original() {
    let mut app = pair();
    app.press((150.0, 350.0)).hold(ALT).drag_to((150.0, 550.0));
    let guides = app.app().guides();
    assert_eq!(
        lines(&guides, Vertical),
        [(Left, Left, 100.0), (Right, Right, 200.0)]
    );
    assert_eq!(guides.alignment[0].candidate.as_str(), "a:origin");
}

#[test]
fn a_resize_confirms_only_the_edges_its_handle_moves() {
    // `a`'s top is level with `b`'s the whole time. Dragging the right
    // handle says nothing of it; the guide is for the right edge reaching
    // `b`'s left, and `b`'s middle is off the grid.
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("b", Rect::new(400.0, 100.0, 100.0, 300.0)),
    ]);
    app.viewport(VIEWPORT);
    app.select(&["a"]);
    app.press((200.0, 150.0)).drag_to((400.0, 150.0));
    let guides = app.app().guides();
    assert_eq!(lines(&guides, Horizontal), []);
    assert_eq!(lines(&guides, Vertical), [(Right, Left, 400.0)]);
    app.release();
    assert!(app.app().guides().is_empty());
    assert_eq!(app.rect("a"), Rect::new(100.0, 100.0, 300.0, 100.0));
}

#[test]
fn equal_gaps_either_side_show_a_distribution_guide() {
    let mut app = TestApp::with_entities([
        shape("left", Rect::new(100.0, 100.0, 100.0, 100.0)),
        shape("right", Rect::new(500.0, 100.0, 100.0, 100.0)),
        shape("a", Rect::new(300.0, 400.0, 100.0, 100.0)),
    ]);
    app.viewport(VIEWPORT);
    app.press((350.0, 450.0)).drag_to((350.0, 150.0));
    let guides = app.app().guides();
    let [run] = &guides.distribution[..] else {
        panic!("one run, found {:?}", guides.distribution);
    };
    assert_eq!((run.axis, run.gap), (Horizontal, 100.0));
    let gaps: Vec<_> = (run.gaps.iter())
        .map(|gap| (gap.start, gap.end, gap.cross))
        .collect();
    assert_eq!(gaps, [(200.0, 300.0, 150.0), (400.0, 500.0, 150.0)]);
    // A centre only shows where the grid allows it: these are 100 wide.
    assert_eq!(lines(&guides, Vertical), []);
}
