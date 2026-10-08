//! Alignment and distribution guides during a move and a resize (ADR 0012):
//! they confirm what the grid has already lined up, and never pull.

use specular_doc::Rect;
use specular_interact::{GuideAxis, GuideReference, Guides};
use specular_testkit::{ALT, SHIFT, TestApp, group, inside, shape};

use GuideAxis::{Horizontal, Vertical};
use GuideReference::{Bottom, HCenter, Left, Right, Top};

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
fn a_guide_tolerates_half_a_unit_and_no_more() {
    // Shift keeps `a`'s y exactly as it is, so its top is `off` below `b`'s.
    let tops = |off: f64| {
        let mut app = TestApp::with_entities([
            shape("a", Rect::new(100.0, 105.0 + off, 100.0, 100.0)),
            shape("b", Rect::new(400.0, 105.0, 100.0, 100.0)),
        ]);
        app.viewport(VIEWPORT);
        app.press((150.0, 155.0))
            .hold(SHIFT)
            .drag_to((210.0, 155.0));
        lines(&app.app().guides(), Horizontal)
    };
    assert_eq!(tops(0.5), [(Top, Top, 105.0), (Bottom, Bottom, 205.0)]);
    assert_eq!(tops(0.6), []);
}

#[test]
fn a_centre_is_kept_unless_both_edges_agree() {
    // `a` is twice as tall as `b`: its top is on `b`'s top and its middle on
    // `b`'s bottom, but its bottom is on nothing.
    let mut app = TestApp::with_entities([
        shape("a", Rect::new(100.0, 300.0, 100.0, 200.0)),
        shape("b", Rect::new(400.0, 100.0, 100.0, 100.0)),
    ]);
    app.viewport(VIEWPORT);
    app.press((150.0, 400.0)).drag_to((150.0, 200.0));
    assert_eq!(
        lines(&app.app().guides(), Horizontal),
        [(Top, Top, 100.0), (HCenter, Bottom, 200.0)]
    );
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
    app.wheel((-400.0, 0.0));
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
fn every_handle_confirms_the_edges_it_moves_and_no_others() {
    // `a` is 100 square at (300, 300). Each handle is dragged out, then the
    // resized rect has a neighbour level with both its top and bottom and
    // another level with both its left and right.
    type Case = ((f32, f32), (f32, f32), Rect, &'static [GuideReference]);
    let cases: [Case; 8] = [
        (
            (300.0, 300.0),
            (200.0, 200.0),
            Rect::new(200.0, 200.0, 200.0, 200.0),
            &[Top, Left],
        ),
        (
            (400.0, 300.0),
            (500.0, 200.0),
            Rect::new(300.0, 200.0, 200.0, 200.0),
            &[Top, Right],
        ),
        (
            (400.0, 400.0),
            (500.0, 500.0),
            Rect::new(300.0, 300.0, 200.0, 200.0),
            &[Bottom, Right],
        ),
        (
            (300.0, 400.0),
            (200.0, 500.0),
            Rect::new(200.0, 300.0, 200.0, 200.0),
            &[Bottom, Left],
        ),
        (
            (350.0, 300.0),
            (350.0, 200.0),
            Rect::new(300.0, 200.0, 100.0, 200.0),
            &[Top],
        ),
        (
            (400.0, 350.0),
            (500.0, 350.0),
            Rect::new(300.0, 300.0, 200.0, 100.0),
            &[Right],
        ),
        (
            (350.0, 400.0),
            (350.0, 500.0),
            Rect::new(300.0, 300.0, 100.0, 200.0),
            &[Bottom],
        ),
        (
            (300.0, 350.0),
            (200.0, 350.0),
            Rect::new(200.0, 300.0, 200.0, 100.0),
            &[Left],
        ),
    ];
    for (from, to, r, moved) in cases {
        let mut app = TestApp::with_entities([
            shape("a", Rect::new(300.0, 300.0, 100.0, 100.0)),
            shape("h", Rect::new(900.0, r.y, 100.0, r.height)),
            shape("v", Rect::new(r.x, 700.0, r.width, 100.0)),
        ]);
        app.viewport(VIEWPORT);
        app.select(&["a"]);
        app.press(from).drag_to(to);
        assert_eq!(app.rect("a"), r, "{from:?}");
        let guides = app.app().guides();
        let want = |along: [GuideReference; 2]| -> Vec<Line> {
            along
                .into_iter()
                .filter(|reference| moved.contains(reference))
                .map(|reference| {
                    let edge = match reference {
                        Top => r.y,
                        Bottom => r.y + r.height,
                        Left => r.x,
                        _ => r.x + r.width,
                    };
                    (reference, reference, edge)
                })
                .collect()
        };
        assert_eq!(lines(&guides, Horizontal), want([Top, Bottom]), "{from:?}");
        assert_eq!(lines(&guides, Vertical), want([Left, Right]), "{from:?}");
    }
}

#[test]
fn a_dragged_groups_members_are_spoken_for_by_the_group() {
    // `m` is level with `b`, but it moves with `g`, whose own edges are not.
    let mut app = TestApp::with_entities([
        group("g", Rect::new(100.0, 300.0, 200.0, 200.0)),
        inside("g", shape("m", Rect::new(120.0, 320.0, 100.0, 100.0))),
        shape("b", Rect::new(600.0, 320.0, 100.0, 100.0)),
    ]);
    app.viewport(VIEWPORT);
    app.select(&["g"]);
    app.press((250.0, 480.0)).drag_to((260.0, 480.0));
    assert_eq!(lines(&app.app().guides(), Horizontal), []);
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
