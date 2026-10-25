//! Which guides a resize confirms, handle by handle (ADR 0012). The guides
//! of moves, copies and the edge cases are scenarios (`guides.txt`,
//! `guides-cases.txt`).

use specular_doc::Rect;
use specular_interact::{GuideAxis, GuideReference, Guides};
use specular_testkit::{TestApp, shape};

use GuideAxis::{Horizontal, Vertical};
use GuideReference::{Bottom, Left, Right, Top};

const VIEWPORT: (f32, f32) = (1600.0, 1000.0);

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
