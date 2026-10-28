//! Laying out screens as a flow: the screens never overlap, each edge runs
//! at right angles from a side of one to a side of the other, through no
//! screen and along no other edge, and in a flow with no loop every arrow
//! runs the same way.
#![expect(clippy::panic, reason = "a helper fails the test it is called from")]

use specular_doc::{Document, Edge, EdgeSide, EntityId, Rect};
use specular_interact::{FlowDirection, flow_plan, plan_commands};
use specular_testkit::{connected, document, file};

const PHONE: (f64, f64) = (390.0, 844.0);
const DESKTOP: (f64, f64) = (1440.0, 900.0);

/// Screens side by side in a row, as an agent adds them.
fn screens(sizes: &[(&str, (f64, f64))]) -> Document {
    let mut x = 0.0;
    document(sizes.iter().map(|(id, (width, height))| {
        let entity = file(id, Rect::new(x, 0.0, *width, *height));
        x += width + 40.0;
        entity
    }))
}

fn linked(mut document: Document, links: &[(&str, &str)]) -> Document {
    for (index, (from, to)) in links.iter().enumerate() {
        document = connected(document, &format!("e{index}"), from, to);
    }
    document
}

fn laid_out(document: &Document, direction: FlowDirection) -> Document {
    let ids: Vec<EntityId> = document
        .entities()
        .map(|entity| entity.id.clone())
        .collect();
    let Some(plan) = flow_plan(document, &ids, direction, None) else {
        panic!("no plan for {ids:?}");
    };
    let mut after = document.clone();
    for command in plan_commands(document, &plan) {
        if let Err(error) = after.apply(command) {
            panic!("the plan does not apply: {error}");
        }
    }
    after
}

fn rect(document: &Document, id: &EntityId) -> Rect {
    match document.entity(id) {
        Some(entity) => entity.rect,
        None => panic!("no entity {id}"),
    }
}

/// Whether two coordinates are the same, to well under a pixel.
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn on_side(rect: Rect, side: EdgeSide, [x, y]: [f64; 2]) -> bool {
    let along_x = x >= rect.x && x <= rect.x + rect.width;
    let along_y = y >= rect.y && y <= rect.y + rect.height;
    match side {
        EdgeSide::Top => along_x && same(y, rect.y),
        EdgeSide::Bottom => along_x && same(y, rect.y + rect.height),
        EdgeSide::Left => along_y && same(x, rect.x),
        EdgeSide::Right => along_y && same(x, rect.x + rect.width),
    }
}

/// Whether the segment `a` to `b`, which is level or upright, passes
/// inside `rect`.
fn passes_through(rect: Rect, a: [f64; 2], b: [f64; 2]) -> bool {
    let (low_x, high_x) = (a[0].min(b[0]), a[0].max(b[0]));
    let (low_y, high_y) = (a[1].min(b[1]), a[1].max(b[1]));
    low_x < rect.x + rect.width - 1.0
        && high_x > rect.x + 1.0
        && low_y < rect.y + rect.height - 1.0
        && high_y > rect.y + 1.0
}

/// Whether two segments lie on one line and share more than a point.
fn runs_along(a: ([f64; 2], [f64; 2]), b: ([f64; 2], [f64; 2])) -> bool {
    let overlap = |p: f64, q: f64, r: f64, s: f64| p.max(q).min(r.max(s)) - p.min(q).max(r.min(s));
    let level = |s: ([f64; 2], [f64; 2])| same(s.0[1], s.1[1]);
    let upright = |s: ([f64; 2], [f64; 2])| same(s.0[0], s.1[0]);
    (level(a) && level(b) && same(a.0[1], b.0[1]) && overlap(a.0[0], a.1[0], b.0[0], b.1[0]) > 1.0)
        || (upright(a)
            && upright(b)
            && same(a.0[0], b.0[0])
            && overlap(a.0[1], a.1[1], b.0[1], b.1[1]) > 1.0)
}

/// The runs of `edge`'s route, which every edge of a flow has.
fn segments(edge: &Edge) -> Vec<([f64; 2], [f64; 2])> {
    let Some(route) = &edge.route else {
        panic!("{} has no route", edge.id);
    };
    route.windows(2).map(|pair| (pair[0], pair[1])).collect()
}

/// A flow to lay out: screens by name and size, the links between them,
/// and whether every arrow should run one way.
struct Case {
    name: &'static str,
    screens: Vec<(&'static str, (f64, f64))>,
    links: Vec<(&'static str, &'static str)>,
    one_way: bool,
}

/// Lays `case` out running `direction` and checks every rule.
fn check(case: &Case, direction: FlowDirection) {
    let at = format!("{} ({direction:?})", case.name);
    let after = laid_out(&linked(screens(&case.screens), &case.links), direction);
    let rects: Vec<(EntityId, Rect)> = (after.entities())
        .map(|entity| (entity.id.clone(), entity.rect))
        .collect();
    for (i, (a, first)) in rects.iter().enumerate() {
        for (b, second) in &rects[i + 1..] {
            let apart = first.x + first.width <= second.x
                || second.x + second.width <= first.x
                || first.y + first.height <= second.y
                || second.y + second.height <= first.y;
            assert!(apart, "{at}: {a} and {b} overlap");
        }
    }
    let edges: Vec<&Edge> = after.edges().collect();
    for edge in &edges {
        let runs = segments(edge);
        let (from, to) = (rect(&after, &edge.from), rect(&after, &edge.to));
        let (first, last) = (runs[0].0, runs[runs.len() - 1].1);
        let id = &edge.id;
        let (Some(from_side), Some(to_side)) = (edge.from_side, edge.to_side) else {
            panic!("{at}: {id} names no sides");
        };
        assert!(
            on_side(from, from_side, first),
            "{at}: {id} leaves its side"
        );
        assert!(on_side(to, to_side, last), "{at}: {id} reaches its side");
        for (a, b) in runs {
            let square = same(a[0], b[0]) || same(a[1], b[1]);
            assert!(square, "{at}: {id} turns at a slant");
            for (other, rect) in &rects {
                let through = passes_through(*rect, a, b);
                assert!(!through, "{at}: {id} runs through {other}");
            }
        }
        if case.one_way {
            let ahead = match direction {
                FlowDirection::Across => to.x >= from.x + from.width,
                FlowDirection::Down => to.y >= from.y + from.height,
            };
            assert!(ahead, "{at}: {id} runs against the flow");
        }
    }
    for (i, a) in edges.iter().enumerate() {
        for b in &edges[i + 1..] {
            for one in segments(a) {
                for two in segments(b) {
                    assert!(!runs_along(one, two), "{at}: {} runs along {}", a.id, b.id);
                }
            }
        }
    }
}

#[test]
fn a_flow_places_screens_apart_and_routes_every_edge_clear() {
    let phone = |id| (id, PHONE);
    let cases = [
        Case {
            name: "a straight run",
            screens: vec![phone("a"), phone("b"), phone("c")],
            links: vec![("a", "b"), ("b", "c")],
            one_way: true,
        },
        Case {
            name: "a branch that joins again",
            screens: vec![phone("a"), phone("b"), phone("c"), phone("d")],
            links: vec![("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")],
            one_way: true,
        },
        Case {
            name: "a checkout with a retry and a way back",
            screens: ["cart", "ship", "pay", "err", "done"].map(phone).to_vec(),
            links: vec![
                ("cart", "ship"),
                ("ship", "pay"),
                ("pay", "err"),
                ("err", "pay"),
                ("pay", "done"),
                ("err", "cart"),
            ],
            one_way: false,
        },
        Case {
            name: "an edge that skips a rank",
            screens: vec![phone("a"), phone("b"), phone("c"), phone("d")],
            links: vec![("a", "b"), ("b", "c"), ("c", "d"), ("a", "d"), ("a", "c")],
            one_way: true,
        },
        Case {
            name: "phones and desktops together",
            screens: vec![
                phone("home"),
                ("site", DESKTOP),
                phone("menu"),
                ("cart", DESKTOP),
            ],
            links: vec![
                ("home", "menu"),
                ("home", "site"),
                ("site", "cart"),
                ("menu", "cart"),
            ],
            one_way: true,
        },
    ];
    for direction in [FlowDirection::Across, FlowDirection::Down] {
        for case in &cases {
            check(case, direction);
        }
    }
}
