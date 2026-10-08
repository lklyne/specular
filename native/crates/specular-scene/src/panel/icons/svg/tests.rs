use super::*;

fn at(x: f32, y: f32) -> Point {
    Point::new(x, y)
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3
}

#[test]
fn absolute_and_relative_lines_and_a_close() {
    assert_eq!(
        parse("M1 2h10v10H1z"),
        [
            PathCommand::MoveTo(at(1.0, 2.0)),
            PathCommand::LineTo(at(11.0, 2.0)),
            PathCommand::LineTo(at(11.0, 12.0)),
            PathCommand::LineTo(at(1.0, 12.0)),
            PathCommand::Close,
        ]
    );
    // A close puts the pen back where the sub-path began.
    assert_eq!(
        parse("M1 1h4zl1 1"),
        [
            PathCommand::MoveTo(at(1.0, 1.0)),
            PathCommand::LineTo(at(5.0, 1.0)),
            PathCommand::Close,
            PathCommand::LineTo(at(2.0, 2.0)),
        ]
    );
}

#[test]
fn numbers_after_a_move_are_lines_and_signs_split_them() {
    assert_eq!(
        parse("m12 19-7-7 7-7M4.5 4.5 19 19h.01"),
        [
            PathCommand::MoveTo(at(12.0, 19.0)),
            PathCommand::LineTo(at(5.0, 12.0)),
            PathCommand::LineTo(at(12.0, 5.0)),
            PathCommand::MoveTo(at(4.5, 4.5)),
            PathCommand::LineTo(at(19.0, 19.0)),
            PathCommand::LineTo(at(19.01, 19.0)),
        ]
    );
}

#[test]
fn a_smooth_cubic_mirrors_the_control_before_it() {
    let commands = parse("M0 0c1 0 2 1 2 2s1 2 2 2S9 9 8 8");
    assert_eq!(
        commands[2..],
        [
            PathCommand::CubicTo {
                control1: at(2.0, 3.0),
                control2: at(3.0, 4.0),
                to: at(4.0, 4.0),
            },
            PathCommand::CubicTo {
                control1: at(5.0, 4.0),
                control2: at(9.0, 9.0),
                to: at(8.0, 8.0),
            },
        ]
    );
}

/// The control points and end of each cubic an arc became.
fn cubics(commands: &[PathCommand]) -> Vec<[Point; 3]> {
    commands
        .iter()
        .filter_map(|command| match command {
            PathCommand::CubicTo {
                control1,
                control2,
                to,
            } => Some([*control1, *control2, *to]),
            _ => None,
        })
        .collect()
}

/// The ends of the cubics an arc became.
fn ends(commands: &[PathCommand]) -> Vec<Point> {
    commands
        .iter()
        .filter_map(|command| match command {
            PathCommand::CubicTo { to, .. } => Some(*to),
            _ => None,
        })
        .collect()
}

#[test]
fn a_half_circle_is_two_quarter_cubics_through_its_top() {
    let commands = parse("M0 50A50 50 0 1 1 100 50");
    let ends = ends(&commands);
    assert_eq!(ends.len(), 2);
    assert!(near(ends[0], at(50.0, 0.0)), "{ends:?}");
    assert_eq!(ends[1], at(100.0, 50.0));
    // A quarter circle's handles are 0.5523 of its radius long.
    let first = cubics(&commands)[0];
    assert!(near(first[0], at(0.0, 22.385_77)), "{first:?}");
    assert!(near(first[1], at(22.385_77, 0.0)), "{first:?}");
}

#[test]
fn the_flags_pick_which_of_two_small_arcs_is_drawn() {
    let clockwise = cubics(&parse("M0 0A10 10 0 0 1 10 10"));
    let anticlockwise = cubics(&parse("M0 0A10 10 0 0 0 10 10"));
    let expect = |cubic: [Point; 3], c1: Point, c2: Point| {
        assert!(near(cubic[0], c1) && near(cubic[1], c2), "{cubic:?}");
        assert_eq!(cubic[2], at(10.0, 10.0));
    };
    assert_eq!(clockwise.len() + anticlockwise.len(), 2);
    expect(clockwise[0], at(5.522_85, 0.0), at(10.0, 4.477_15));
    expect(anticlockwise[0], at(0.0, 5.522_85), at(4.477_15, 10.0));
}

#[test]
fn an_arc_with_radii_too_small_is_grown_to_reach() {
    let ends = ends(&parse("M0 0A1 1 0 0 1 10 0"));
    assert_eq!(ends.last(), Some(&at(10.0, 0.0)));
    assert!(near(ends[0], at(5.0, -5.0)), "{ends:?}");
    // Grown to a half circle of radius 5 about (5, 0), in both directions.
    let first = cubics(&parse("M0 0A1 1 0 0 1 10 0"))[0];
    assert!(near(first[0], at(0.0, -2.761_42)), "{first:?}");
    assert!(near(first[1], at(2.238_58, -5.0)), "{first:?}");
}

#[test]
fn nonsense_ends_the_path_where_it_stops() {
    assert_eq!(parse("M1 1L2"), [PathCommand::MoveTo(at(1.0, 1.0))]);
    assert_eq!(parse("M1 1X"), [PathCommand::MoveTo(at(1.0, 1.0))]);
    assert_eq!(parse("M1 1X L2 2"), [PathCommand::MoveTo(at(1.0, 1.0))]);
    assert_eq!(parse(""), []);
}

#[test]
fn quadratics_and_their_smooth_form_mirror_the_last_control() {
    assert_eq!(
        parse("M0 0Q1 1 2 0T4 0q1 1 2 0t2 0"),
        [
            PathCommand::MoveTo(at(0.0, 0.0)),
            PathCommand::QuadTo {
                control: at(1.0, 1.0),
                to: at(2.0, 0.0),
            },
            PathCommand::QuadTo {
                control: at(3.0, -1.0),
                to: at(4.0, 0.0),
            },
            PathCommand::QuadTo {
                control: at(5.0, 1.0),
                to: at(6.0, 0.0),
            },
            PathCommand::QuadTo {
                control: at(7.0, -1.0),
                to: at(8.0, 0.0),
            },
        ]
    );
}

#[test]
fn exponents_and_flags_run_into_the_numbers_after_them() {
    assert_eq!(
        parse("M1e1 2E-1h.5.5"),
        [
            PathCommand::MoveTo(at(10.0, 0.2)),
            PathCommand::LineTo(at(10.5, 0.2)),
            PathCommand::LineTo(at(11.0, 0.2)),
        ]
    );
    // `0110` is large 0, sweep 1, then 10: a half circle over the top.
    let arc = parse("M0 0a5 5 0 0110 0");
    assert_eq!(arc, parse("M0 0a5 5 0 0 1 10 0"));
    assert!(near(ends(&arc)[0], at(5.0, -5.0)), "{arc:?}");
}
