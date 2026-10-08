//! The outline of a freehand stroke: a port of `perfect-freehand`'s
//! `getStroke` for the one set of options the drawing layer uses (no
//! thinning, no taper, no simulated pressure, a finished stroke).
//!
//! The input is the pointer path; the output is a closed polygon around it,
//! `size` wide, with round or flat ends. Loops and constants follow the
//! library step for step so a stroke has the same shape in both apps.

use std::f64::consts::PI;

use glam::DVec2;

/// How much of the way to each input point the smoothed path moves.
const STREAMLINE: f64 = 0.75;
/// Outline points closer than `size * SMOOTHING` to the last one are dropped.
const SMOOTHING: f64 = 0.8;
/// A hair over pi, so a half turn lands past the opposite side instead of
/// short of it.
const FIXED_PI: f64 = PI + 0.0001;
const START_CAP_SEGMENTS: f64 = 13.0;
const END_CAP_SEGMENTS: f64 = 29.0;
const CORNER_CAP_SEGMENTS: f64 = 13.0;
/// Points this close to the end of the stroke, in path units, are noise.
const END_NOISE_THRESHOLD: f64 = 3.0;
const MIN_RADIUS: f64 = 0.01;

/// One point of the smoothed path.
#[derive(Debug, Clone, Copy)]
struct StrokePoint {
    point: DVec2,
    /// Unit vector from this point back to the previous one.
    vector: DVec2,
    /// Path length from the first point to this one.
    running_length: f64,
}

/// The closed outline of a stroke through `points`, `size` wide. `cap`
/// rounds the two ends; without it they are cut flat. Empty for no points or
/// a non-positive size.
pub(crate) fn outline(points: &[DVec2], size: f64, cap: bool) -> Vec<DVec2> {
    outline_points(&stroke_points(points, size), size, cap)
}

/// The input path smoothed towards each point, with the direction and
/// running length the outline needs.
fn stroke_points(input: &[DVec2], size: f64) -> Vec<StrokePoint> {
    let Some(&first) = input.first() else {
        return Vec::new();
    };
    let t = 0.15 + (1.0 - STREAMLINE) * 0.85;
    // Two points get three more between them, and one point gets a
    // neighbour, so there is always a direction to work with.
    let points: Vec<DVec2> = match input {
        [only] => vec![*only, *only + DVec2::ONE],
        [from, to] => (0..5_u8)
            .map(|step| from.lerp(*to, f64::from(step) / 4.0))
            .collect(),
        _ => input.to_vec(),
    };
    let mut out = vec![StrokePoint {
        point: first,
        vector: DVec2::ONE,
        running_length: 0.0,
    }];
    let mut previous = first;
    let mut running_length = 0.0;
    let mut reached_minimum_length = false;
    let last = points.len() - 1;
    for (index, &raw) in points.iter().enumerate().skip(1) {
        let point = if index == last {
            raw
        } else {
            previous.lerp(raw, t)
        };
        if point == previous {
            continue;
        }
        running_length += point.distance(previous);
        // The start waits until the path has gone `size` from its origin.
        if index < last && !reached_minimum_length {
            if running_length < size {
                continue;
            }
            reached_minimum_length = true;
        }
        out.push(StrokePoint {
            point,
            vector: (previous - point).normalize(),
            running_length,
        });
        previous = point;
    }
    out[0].vector = out.get(1).map_or(DVec2::ZERO, |second| second.vector);
    out
}

/// `(y, -x)`: the vector turned a quarter turn.
fn per(vector: DVec2) -> DVec2 {
    DVec2::new(vector.y, -vector.x)
}

fn rotate_around(point: DVec2, centre: DVec2, radians: f64) -> DVec2 {
    let (sin, cos) = radians.sin_cos();
    let local = point - centre;
    centre + DVec2::new(local.x * cos - local.y * sin, local.x * sin + local.y * cos)
}

/// The values `step, 2*step, ..` up to `end`, accumulated the way the
/// library's `for` loops do so the same number of points comes out.
fn steps(start: f64, step: f64, include_end: bool) -> impl Iterator<Item = f64> {
    let mut t = start;
    std::iter::from_fn(move || {
        let current = t;
        let inside = if include_end {
            current <= 1.0
        } else {
            current < 1.0
        };
        t += step;
        inside.then_some(current)
    })
}

fn outline_points(points: &[StrokePoint], size: f64, cap: bool) -> Vec<DVec2> {
    let (Some(first), Some(last)) = (points.first(), points.last()) else {
        return Vec::new();
    };
    if size <= 0.0 {
        return Vec::new();
    }
    let total_length = last.running_length;
    let min_distance = (size * SMOOTHING).powi(2);
    let radius = (size / 2.0).max(MIN_RADIUS);

    let mut left: Vec<DVec2> = Vec::new();
    let mut right: Vec<DVec2> = Vec::new();
    let mut previous_vector = first.vector;
    let mut previous_left = first.point;
    let mut previous_right = first.point;
    let mut previous_was_sharp = false;

    for (index, current) in points.iter().enumerate() {
        let is_last = index == points.len() - 1;
        if !is_last && total_length - current.running_length < END_NOISE_THRESHOLD {
            continue;
        }
        let StrokePoint { point, vector, .. } = *current;
        let next_vector = if is_last {
            vector
        } else {
            points[index + 1].vector
        };
        let next_dot = if is_last {
            1.0
        } else {
            vector.dot(next_vector)
        };
        let is_sharp = vector.dot(previous_vector) < 0.0 && !previous_was_sharp;
        let next_is_sharp = next_dot < 0.0;

        if is_sharp || next_is_sharp {
            // More than a right angle: a round cap at the corner.
            let offset = per(previous_vector) * radius;
            for t in steps(0.0, 1.0 / CORNER_CAP_SEGMENTS, true) {
                previous_left = rotate_around(point - offset, point, FIXED_PI * t);
                left.push(previous_left);
                previous_right = rotate_around(point + offset, point, -FIXED_PI * t);
                right.push(previous_right);
            }
            if next_is_sharp {
                previous_was_sharp = true;
            }
            continue;
        }
        previous_was_sharp = false;

        if is_last {
            let offset = per(vector) * radius;
            left.push(point - offset);
            right.push(point + offset);
            continue;
        }

        let offset = per(next_vector.lerp(vector, next_dot)) * radius;
        let (to_left, to_right) = (point - offset, point + offset);
        if index <= 1 || previous_left.distance_squared(to_left) > min_distance {
            left.push(to_left);
            previous_left = to_left;
        }
        if index <= 1 || previous_right.distance_squared(to_right) > min_distance {
            right.push(to_right);
            previous_right = to_right;
        }
        previous_vector = vector;
    }

    if points.len() == 1 {
        return dot(first.point, radius);
    }
    let (Some(&first_left), Some(&first_right)) = (left.first(), right.first()) else {
        return Vec::new();
    };
    let start_cap = if cap {
        steps(1.0 / START_CAP_SEGMENTS, 1.0 / START_CAP_SEGMENTS, true)
            .map(|t| rotate_around(first_right, first.point, FIXED_PI * t))
            .collect()
    } else {
        flat_start_cap(first.point, first_left, first_right)
    };
    let direction = per(-last.vector);
    let end_cap: Vec<DVec2> = if cap {
        let start = last.point + direction * radius;
        steps(1.0 / END_CAP_SEGMENTS, 1.0 / END_CAP_SEGMENTS, false)
            .map(|t| rotate_around(start, last.point, FIXED_PI * 3.0 * t))
            .collect()
    } else {
        vec![
            last.point + direction * radius,
            last.point + direction * (radius * 0.99),
            last.point - direction * (radius * 0.99),
            last.point - direction * radius,
        ]
    };

    // Out along the left side, round the end, back along the right side,
    // round the start.
    left.extend(end_cap);
    left.extend(right.into_iter().rev());
    left.extend(start_cap);
    left
}

fn flat_start_cap(centre: DVec2, left: DVec2, right: DVec2) -> Vec<DVec2> {
    let corners = left - right;
    let (near, far) = (corners * 0.5, corners * 0.51);
    vec![centre - near, centre - far, centre + far, centre + near]
}

/// A circle of points, for a stroke that never left its first point.
fn dot(centre: DVec2, radius: f64) -> Vec<DVec2> {
    let start = centre - per(-DVec2::ONE).normalize() * radius;
    steps(1.0 / START_CAP_SEGMENTS, 1.0 / START_CAP_SEGMENTS, true)
        .map(|t| rotate_around(start, centre, FIXED_PI * 2.0 * t))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Point count, coordinate sums, first and last point, rounded to a
    /// millionth: enough to pin the whole outline against the library's.
    fn summary(points: &[(f64, f64)], size: f64, cap: bool) -> (usize, [f64; 2], [f64; 4]) {
        let input: Vec<DVec2> = points.iter().map(|&(x, y)| DVec2::new(x, y)).collect();
        let out = outline(&input, size, cap);
        let round = |value: f64| (value * 1e6).round() / 1e6;
        let sum = out.iter().fold(DVec2::ZERO, |sum, point| sum + *point);
        let (first, last) = (out[0], out[out.len() - 1]);
        (
            out.len(),
            [round(sum.x), round(sum.y)],
            [round(first.x), round(first.y), round(last.x), round(last.y)],
        )
    }

    // The expected values are `getStroke` from perfect-freehand 1.2 run on
    // the same input with the drawing layer's options.

    #[test]
    fn a_curved_line_matches_the_library() {
        let points = [
            (0.0, 0.0),
            (40.0, 0.0),
            (80.0, 10.0),
            (120.0, 40.0),
            (160.0, 40.0),
        ];
        assert_eq!(
            summary(&points, 8.0, true),
            (52, [5_190.988_665, 1286.7085], [0.0, -4.0, 0.0004, -4.0])
        );
    }

    #[test]
    fn one_point_is_a_dot_sized_stroke() {
        assert_eq!(
            summary(&[(5.0, 5.0)], 4.0, true),
            (
                44,
                [238.742_628, 255.214_904],
                [7.414_214, 4.585_786, 5.414_455, 2.585_828]
            )
        );
    }

    #[test]
    fn a_hairpin_gets_a_round_corner() {
        let points = [
            (0.0, 0.0),
            (50.0, 0.0),
            (100.0, 0.0),
            (50.0, 2.0),
            (0.0, 4.0),
            (-40.0, 6.0),
        ];
        assert_eq!(
            summary(&points, 8.0, true),
            (80, [322.993_017, 203.528_39], [0.0, -4.0, 0.0004, -4.0])
        );
    }

    #[test]
    fn nothing_comes_from_no_points_or_no_width() {
        assert_eq!(
            (
                outline(&[], 8.0, true).len(),
                outline(&[DVec2::ZERO, DVec2::X], 0.0, true).len()
            ),
            (0, 0)
        );
    }

    /// The points in order, folded into one number: a reordered or shifted
    /// outline changes it where a sum does not.
    fn signature(out: &[DVec2]) -> f64 {
        let folded: f64 = (out.iter().enumerate())
            .map(|(index, point)| (index + 1) as f64 * (point.x * 7.0 + point.y * 13.0))
            .sum();
        (folded * 1e4).round() / 1e4
    }

    const PIN_DENSE: (usize, f64) = (76, 1_052_378.688);
    const PIN_DOT_LEN: usize = 13;
    const PIN_DOT: (f64, f64, f64) = (6.909_451, 4.405_023, 8_298.987_4);

    // These two are pinned from this port; the rules they cover are the
    // library's, but the numbers were not run through it.

    #[test]
    fn a_dense_path_with_a_hairpin_end_keeps_its_outline_order_and_drops_close_points() {
        let points: Vec<DVec2> = (0..60_u8)
            .map(|step| {
                let x = f64::from(step) * 1.5;
                DVec2::new(x, 12.0 * (x / 20.0).sin())
            })
            .chain([
                DVec2::new(90.0, 4.0),
                DVec2::new(93.0, 4.0),
                DVec2::new(95.0, 4.0),
                DVec2::new(91.0, 4.5),
            ])
            .collect();
        let out = outline(&points, 8.0, true);
        assert_eq!((out.len(), signature(&out)), PIN_DENSE);
    }

    #[test]
    fn two_points_that_never_part_make_a_circle_round_the_first() {
        let at = DVec2::new(5.0, 5.0);
        let out = outline(&[at, at], 4.0, true);
        assert_eq!(out.len(), PIN_DOT_LEN);
        assert!(out.iter().all(|p| ((p.distance(at)) - 2.0).abs() < 1e-9));
        let round = |value: f64| (value * 1e6).round() / 1e6;
        assert_eq!((round(out[0].x), round(out[0].y), signature(&out)), PIN_DOT);
    }
}
