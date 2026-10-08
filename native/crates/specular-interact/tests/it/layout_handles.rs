//! A line's reorder dots and gap strips (ADR 0015): where they show, and
//! what dragging one does to a managed group and to a loose even selection.

use glam::{DVec2, Vec2};
use specular_doc::{Entity, Group, Kind, LayoutMode, Rect};
use specular_interact::{Hit, Key, LayoutAxis, LayoutHandle, hit_test};
use specular_testkit::{TestApp, group, inside, shape};

const A: Rect = Rect::new(100.0, 100.0, 100.0, 100.0);
const B: Rect = Rect::new(240.0, 100.0, 100.0, 100.0);
const C: Rect = Rect::new(380.0, 100.0, 100.0, 100.0);
/// The middles of the three boxes, where their dots are.
const DOT_A: (f32, f32) = (150.0, 150.0);
const DOT_C: (f32, f32) = (430.0, 150.0);
/// A point in the gap between `a` and `b`.
const GAP: (f32, f32) = (220.0, 150.0);

/// `a`, `b` and `c` as the managed row `g`, 40 apart.
fn managed() -> TestApp {
    let row = Entity {
        kind: Kind::Group(Group {
            managed_layout: Some(true),
            layout_mode: Some(LayoutMode::Row),
            layout_gap: Some(40.0),
            ..Group::default()
        }),
        ..group("g", Rect::new(76.0, 76.0, 428.0, 148.0))
    };
    TestApp::with_entities([
        inside("g", shape("a", A)),
        inside("g", shape("b", B)),
        inside("g", shape("c", C)),
        row,
    ])
}

/// The same three boxes with no group, all selected.
fn loose() -> TestApp {
    let mut app = TestApp::with_entities([shape("a", A), shape("b", B), shape("c", C)]);
    app.select(&["a", "b", "c"]);
    app
}

/// Sets a line up.
type Setup = fn() -> TestApp;
/// Name, the line, the dot pressed, the x it is dragged to, and the lefts
/// of `a`, `b` and `c` after, or `None` when nothing moves.
type Reorder = (&'static str, Setup, (f32, f32), f32, Option<[f64; 3]>);
/// Name, the line, the x the strip is dragged to, the lefts after, and the
/// gap the group stores.
type Regap = (&'static str, Setup, f32, [f64; 3], Option<f64>);

const START: [f64; 3] = [100.0, 240.0, 380.0];

fn lefts(app: &TestApp) -> [f64; 3] {
    ["a", "b", "c"].map(|id| app.rect(id).x)
}

fn dots(app: &TestApp) -> Vec<String> {
    (app.app().reorder_dots().iter())
        .map(|dot| dot.entity.to_string())
        .collect()
}

#[test]
fn a_line_shows_its_handles_while_it_or_a_member_is_selected() {
    // (what is selected, the dots, how many gap strips).
    let rows: [(&str, Setup, &[&str], usize); 6] = [
        ("a managed row, nothing selected", managed, &[], 0),
        (
            "a managed row, selected",
            || {
                let mut app = managed();
                app.select(&["g"]);
                app
            },
            &["a", "b", "c"],
            2,
        ),
        (
            "a managed row, one member selected",
            || {
                let mut app = managed();
                app.select(&["b"]);
                app
            },
            &["a", "b", "c"],
            2,
        ),
        ("a loose even row", loose, &["a", "b", "c"], 2),
        (
            "a loose pair",
            || {
                let mut app = loose();
                app.select(&["a", "c"]);
                app
            },
            &["a", "c"],
            1,
        ),
        (
            "a loose uneven row",
            || {
                let mut app = TestApp::with_entities([
                    shape("a", A),
                    shape("b", B),
                    shape("c", Rect::new(500.0, 100.0, 100.0, 100.0)),
                ]);
                app.select(&["a", "b", "c"]);
                app
            },
            &[],
            0,
        ),
    ];
    for (name, setup, want, strips) in rows {
        let app = setup();
        assert_eq!(dots(&app), want, "{name}");
        assert_eq!(app.app().gap_handles().len(), strips, "{name}");
    }
}

#[test]
fn a_dot_is_over_its_box_and_a_strip_fills_the_gap() {
    let app = loose();
    let at = |point: (f32, f32)| hit_test(app.app(), Vec2::from(point));
    assert_eq!(
        at(DOT_A),
        Hit::Layout(LayoutHandle::Reorder { entity: "a".into() })
    );
    assert_eq!(
        at(GAP),
        Hit::Layout(LayoutHandle::Gap {
            group: None,
            axis: LayoutAxis::X
        })
    );
    // Off the dot the box is a body again.
    assert_eq!(at((120.0, 120.0)), Hit::EntityBody { entity: "a".into() });
    let strip = &app.app().gap_handles()[0].rect;
    assert_eq!(
        (strip.min, strip.size),
        (Vec2::new(200.0, 100.0), Vec2::new(40.0, 100.0))
    );
}

#[test]
fn dragging_a_dot_moves_its_box_to_the_slot_under_the_pointer() {
    // (the line, the dot pressed, where it is dragged, the lefts of a, b, c).
    let rows: [Reorder; 4] = [
        // c to the front of a managed row: the sequence changes.
        (
            "managed, to the front",
            selected_c,
            DOT_C,
            120.0,
            Some([240.0, 380.0, 100.0]),
        ),
        // Not yet half way to the next slot: nothing moves.
        ("managed, short of a slot", selected_c, DOT_C, 370.0, None),
        (
            "loose, to the back",
            loose,
            DOT_A,
            450.0,
            Some([380.0, 100.0, 240.0]),
        ),
        (
            "loose, one slot",
            loose,
            DOT_A,
            230.0,
            Some([240.0, 100.0, 380.0]),
        ),
    ];
    for (name, setup, dot, to, want) in rows {
        let mut app = setup();
        app.press(dot).drag_to((to, 150.0));
        // The line already shows where a release would leave it.
        assert_eq!(lefts(&app), want.unwrap_or(START), "{name}, in flight");
        app.release();
        assert_eq!(lefts(&app), want.unwrap_or(START), "{name}");
        if want.is_some() {
            app.assert_undo_returns_to_start();
        } else {
            assert!(!app.app().can_undo(), "{name}: nothing to undo");
        }
    }
}

fn selected_c() -> TestApp {
    let mut app = managed();
    app.select(&["c"]);
    app
}

#[test]
fn a_managed_reorder_is_kept_in_the_stack_order() {
    let mut app = selected_c();
    app.press(DOT_C).drag_to((120.0, 150.0));
    // In flight the stack is as it was, and the strips are between the
    // boxes as they are shown.
    let strips: Vec<f32> = (app.app().gap_handles().iter())
        .map(|strip| strip.rect.min.x)
        .collect();
    assert_eq!(strips, [200.0, 340.0]);
    app.release();
    let order: Vec<&str> = (app.document().children(&"g".into()))
        .map(|child| child.id.as_str())
        .collect();
    assert_eq!(order, ["c", "a", "b"]);
}

#[test]
fn the_box_being_reordered_floats_under_the_pointer_without_a_dot() {
    let mut app = loose();
    app.press(DOT_A).drag_to((450.0, 190.0));
    let ghost = app
        .app()
        .reorder_ghost()
        .unwrap_or_else(|| panic!("a ghost"));
    // The document has `a` in the last slot, at (380, 100). The pointer has
    // taken it 300 right and 40 down from (100, 100), so it floats at
    // (400, 140).
    assert_eq!(ghost.entity.as_str(), "a");
    assert_eq!(ghost.delta, DVec2::new(20.0, 40.0));
    assert_eq!(dots(&app), ["b", "c"]);
    assert!(app.app().guides().is_empty(), "a line drag shows no guides");
    app.key(Key::Escape);
    assert_eq!(lefts(&app), [100.0, 240.0, 380.0]);
    assert!(!app.app().can_undo());
}

#[test]
fn dragging_a_gap_strip_changes_the_gap_by_the_pointers_travel() {
    let selected_group: Setup = || {
        let mut app = managed();
        app.select(&["g"]);
        app
    };
    // (the line, where the strip is dragged, the lefts after, the gap the
    // group stores).
    let rows: [Regap; 4] = [
        (
            "managed, wider",
            selected_group,
            240.0,
            [100.0, 260.0, 420.0],
            Some(60.0),
        ),
        (
            "managed, closed up",
            selected_group,
            100.0,
            [100.0, 200.0, 300.0],
            Some(0.0),
        ),
        ("loose, narrower", loose, 205.0, [100.0, 225.0, 350.0], None),
        ("loose, wider", loose, 260.0, [100.0, 280.0, 460.0], None),
    ];
    for (name, setup, to, want, stored) in rows {
        let mut app = setup();
        app.press(GAP).drag_to((to, 170.0));
        assert_eq!(lefts(&app), want, "{name}, in flight");
        app.release();
        assert_eq!(lefts(&app), want, "{name}");
        if let Some(gap) = stored {
            let Kind::Group(group) = &app.entity("g").kind else {
                panic!("g is a group");
            };
            assert_eq!(group.layout_gap, Some(gap), "{name}");
        }
        app.assert_undo_returns_to_start();
    }
}
