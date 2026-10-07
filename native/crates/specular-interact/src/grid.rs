//! The canvas grid that drags and placements snap to.

/// The grid pitch in canvas units.
pub(crate) const GRID_SIZE: f64 = 20.0;

/// `value` rounded to the nearest whole number, halves going up. Electron
/// rounds this way, and saved rects have to agree with it.
pub(crate) fn round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// `value` moved to the nearest grid line.
pub(crate) fn snap(value: f64) -> f64 {
    round(value / GRID_SIZE) * GRID_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_goes_to_the_nearest_line_and_halves_go_up() {
        assert_eq!(
            [9.9, 10.0, 29.0, -10.0, -11.0].map(snap),
            [0.0, 20.0, 20.0, 0.0, -20.0]
        );
    }
}
