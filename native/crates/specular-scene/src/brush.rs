//! A brush stroke's outline: a line through points whose width changes along
//! it, narrowing at its ends (`taper`) and where the pointer moved fast
//! (`speed`). A mod's display-list path asks for one (ADR 0057), and the
//! scene fills it as a polygon.

/// How many line widths a tapered end takes to reach full width.
const TAPER_WIDTHS: f64 = 3.0;
/// The narrowest a fast part gets at `speed` 1, as a share of the width.
const FASTEST: f64 = 0.2;
/// The points either side of one that its speed is averaged over.
const SMOOTHING: usize = 2;

/// The closed outline of a line `width` wide through `points`, its left
/// side then its right side back. Empty for fewer than two distinct points.
#[must_use]
pub fn brush_outline(points: &[[f64; 2]], width: f64, taper: bool, speed: f64) -> Vec<[f64; 2]> {
    let points = distinct(points);
    if points.len() < 2 {
        return Vec::new();
    }
    let widths = widths(&points, width, taper, speed.clamp(0.0, 1.0));
    let mut left = Vec::with_capacity(points.len());
    let mut right = Vec::with_capacity(points.len());
    for (i, point) in points.iter().enumerate() {
        let before = points[i.saturating_sub(1)];
        let after = points[(i + 1).min(points.len() - 1)];
        let (dx, dy) = (after[0] - before[0], after[1] - before[1]);
        let length = dx.hypot(dy).max(f64::EPSILON);
        let half = widths[i] / 2.0;
        let (nx, ny) = (-dy / length * half, dx / length * half);
        left.push([point[0] + nx, point[1] + ny]);
        right.push([point[0] - nx, point[1] - ny]);
    }
    left.extend(right.into_iter().rev());
    left
}

fn distinct(points: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut kept: Vec<[f64; 2]> = Vec::with_capacity(points.len());
    for point in points {
        if kept.last() != Some(point) {
            kept.push(*point);
        }
    }
    kept
}

/// The width at each point.
fn widths(points: &[[f64; 2]], width: f64, taper: bool, speed: f64) -> Vec<f64> {
    let gaps: Vec<f64> = (0..points.len())
        .map(|i| {
            let j = if i == 0 { 1 } else { i };
            let ([ax, ay], [bx, by]) = (points[j - 1], points[j]);
            (bx - ax).hypot(by - ay)
        })
        .collect();
    let along: Vec<f64> = gaps
        .iter()
        .scan(0.0, |total, gap| {
            *total += gap;
            Some(*total - gaps[0])
        })
        .collect();
    let total = along.last().copied().unwrap_or(0.0);
    (0..points.len())
        .map(|i| {
            let mut w = width;
            if speed > 0.0 {
                let from = i.saturating_sub(SMOOTHING);
                let to = (i + SMOOTHING).min(points.len() - 1);
                #[expect(clippy::cast_precision_loss, reason = "a handful of points")]
                let gap = gaps[from..=to].iter().sum::<f64>() / (to - from + 1) as f64;
                // A gap of one width keeps the line whole; wider gaps thin it.
                let fast = (gap / width.max(f64::EPSILON) - 1.0).max(0.0);
                let thin = fast / (fast + 1.0);
                w *= 1.0 - speed * (1.0 - FASTEST) * thin;
            }
            if taper {
                let reach = (width * TAPER_WIDTHS).min(total / 2.0).max(f64::EPSILON);
                let end = along[i].min(total - along[i]);
                w *= (end / reach).clamp(0.0, 1.0).sqrt();
            }
            w
        })
        .collect()
}
