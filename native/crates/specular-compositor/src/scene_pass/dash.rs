//! Cuts a polyline into dashes. Pure.

use glam::Vec2;

/// The drawn stretches of `points` under a repeating `on`/`off` pattern, each
/// a polyline. Lengths are in the points' units. A pattern without a positive
/// `on` and `off` leaves the line whole.
pub(crate) fn dashes(points: &[Vec2], on: f32, off: f32) -> Vec<Vec<Vec2>> {
    if !(on > 0.0 && off > 0.0) {
        return vec![points.to_vec()];
    }
    let mut out = Vec::new();
    let mut current: Vec<Vec2> = Vec::new();
    let mut drawing = true;
    let mut left = on;
    for pair in points.windows(2) {
        let (mut from, to) = (pair[0], pair[1]);
        let mut remaining = from.distance(to);
        if drawing && current.is_empty() {
            current.push(from);
        }
        while remaining > left {
            let cut = from + (to - from) * (left / remaining);
            if drawing {
                current.push(cut);
                out.push(std::mem::take(&mut current));
            } else {
                current.push(cut);
            }
            remaining -= left;
            from = cut;
            drawing = !drawing;
            left = if drawing { on } else { off };
        }
        left -= remaining;
        if drawing {
            current.push(to);
        }
    }
    if drawing && current.len() > 1 {
        out.push(current);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(points: &[(f32, f32)]) -> Vec<Vec2> {
        points.iter().map(|&(x, y)| Vec2::new(x, y)).collect()
    }

    #[test]
    fn a_straight_line_is_cut_into_on_stretches() {
        let cut = dashes(&line(&[(0.0, 0.0), (10.0, 0.0)]), 3.0, 2.0);
        assert_eq!(
            cut,
            [
                line(&[(0.0, 0.0), (3.0, 0.0)]),
                line(&[(5.0, 0.0), (8.0, 0.0)])
            ]
        );
    }

    #[test]
    fn a_dash_in_progress_at_the_end_is_kept() {
        let cut = dashes(&line(&[(0.0, 0.0), (7.0, 0.0)]), 3.0, 2.0);
        assert_eq!(cut[1], line(&[(5.0, 0.0), (7.0, 0.0)]));
    }

    #[test]
    fn a_dash_carries_round_a_corner() {
        let cut = dashes(&line(&[(0.0, 0.0), (2.0, 0.0), (2.0, 10.0)]), 4.0, 100.0);
        assert_eq!(cut, [line(&[(0.0, 0.0), (2.0, 0.0), (2.0, 2.0)])]);
    }

    #[test]
    fn a_gap_carries_round_a_corner() {
        let cut = dashes(&line(&[(0.0, 0.0), (3.0, 0.0), (3.0, 10.0)]), 2.0, 3.0);
        assert_eq!(
            cut,
            [
                line(&[(0.0, 0.0), (2.0, 0.0)]),
                line(&[(3.0, 2.0), (3.0, 4.0)]),
                line(&[(3.0, 7.0), (3.0, 9.0)])
            ]
        );
    }
}
