//! A brush stroke's outline (ADR 0057): full width in the middle of a slow
//! line, narrowing to its ends with `taper` and where the points are far
//! apart with `speed`.

use specular_scene::brush_outline;

/// The outline's width at point `i` of a line along the x axis: the gap
/// between its left side and its right side there.
fn width_at(outline: &[[f64; 2]], i: usize) -> f64 {
    let left = outline[i];
    let right = outline[outline.len() - 1 - i];
    (left[1] - right[1]).abs()
}

#[test]
fn a_brush_outline_tapers_its_ends_and_thins_where_fast() {
    let slow: Vec<[f64; 2]> = (0..=40).map(|x| [f64::from(x), 0.0]).collect();
    let plain = brush_outline(&slow, 4.0, false, 0.0);
    assert_eq!(plain.len(), slow.len() * 2, "a side each way");
    assert!((width_at(&plain, 20) - 4.0).abs() < 1e-9, "full width");

    let tapered = brush_outline(&slow, 4.0, true, 0.0);
    assert!(width_at(&tapered, 0) < 0.5, "the press narrows to a point");
    assert!(width_at(&tapered, 40) < 0.5, "and so does the release");
    assert!(
        (width_at(&tapered, 20) - 4.0).abs() < 1e-9,
        "the middle is whole"
    );

    let mut fast = slow.clone();
    fast.extend((1..=5).map(|n| [40.0 + f64::from(n) * 20.0, 0.0]));
    let thinned = brush_outline(&fast, 4.0, false, 1.0);
    assert!(
        (width_at(&thinned, 10) - 4.0).abs() < 1e-9,
        "slow keeps its width"
    );
    assert!(
        width_at(&thinned, fast.len() - 1) < 2.0,
        "fast thins the line"
    );

    assert_eq!(
        brush_outline(&[[1.0, 1.0], [1.0, 1.0]], 4.0, true, 0.0),
        Vec::<[f64; 2]>::new()
    );
}
