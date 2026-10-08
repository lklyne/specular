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
}

#[test]
fn an_arc_with_radii_too_small_is_grown_to_reach() {
    let ends = ends(&parse("M0 0A1 1 0 0 1 10 0"));
    assert_eq!(ends.last(), Some(&at(10.0, 0.0)));
    assert!(near(ends[0], at(5.0, -5.0)), "{ends:?}");
}

#[test]
fn nonsense_ends_the_path_where_it_stops() {
    assert_eq!(parse("M1 1L2"), [PathCommand::MoveTo(at(1.0, 1.0))]);
    assert_eq!(parse("M1 1X"), [PathCommand::MoveTo(at(1.0, 1.0))]);
    assert_eq!(parse(""), []);
}
