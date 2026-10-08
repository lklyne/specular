//! Resize math: the rect a handle drag produces for one entity, and where
//! each entity of a selection lands when the selection's bounds are dragged.
//!
//! Every function works from the rect the drag started with and the canvas
//! point the handle has been dragged to, so a drag that overshoots a limit
//! comes straight back when the pointer does.

use glam::DVec2;
use specular_doc::Rect;

use crate::{Handle, grid};

/// The bounds of a selection cannot be dragged smaller than this, in canvas
/// units.
const MIN_BOUNDS: f64 = 20.0;

/// One axis of a rect under a handle. `sign` is `-1` when the handle moves
/// the low edge, `1` the high edge, `0` neither.
#[derive(Debug, Clone, Copy)]
struct Axis {
    low: f64,
    high: f64,
    sign: f64,
}

/// Where a rect starts and how far it runs along one axis.
type Span = (f64, f64);

impl Axis {
    fn fixed(self) -> f64 {
        if self.sign < 0.0 { self.high } else { self.low }
    }

    fn span(self) -> Span {
        (self.low, self.high - self.low)
    }

    /// The length a handle dragged to `target` asks for, or the current
    /// length when the handle does not move this axis.
    fn asked(self, target: f64, min: f64) -> f64 {
        if self.sign == 0.0 {
            self.high - self.low
        } else {
            ((target - self.fixed()) * self.sign).max(min)
        }
    }

    /// The span `length` long from the fixed edge.
    fn anchored(self, length: f64) -> Span {
        if self.sign < 0.0 {
            (self.high - length, length)
        } else {
            (self.low, length)
        }
    }

    /// As [`anchored`](Self::anchored), with the moving edge on the
    /// nearest grid line that leaves at least `min`.
    fn snapped(self, length: f64, min: f64) -> Span {
        let edge = grid::snap(self.fixed() + self.sign * length);
        if self.sign < 0.0 {
            let low = edge.min(self.high - min);
            (low, self.high - low)
        } else {
            let high = edge.max(self.low + min);
            (self.low, high - self.low)
        }
    }
}

fn axes(rect: Rect, handle: Handle) -> (Axis, Axis) {
    let sign = handle.sign();
    (
        Axis {
            low: rect.x,
            high: rect.x + rect.width,
            sign: sign.x,
        },
        Axis {
            low: rect.y,
            high: rect.y + rect.height,
            sign: sign.y,
        },
    )
}

/// `start` with `handle` dragged to `target`. The moving edges land on the
/// grid and the rect never goes below `min` or flips.
///
/// With `lock` the rect keeps its aspect ratio. A corner follows whichever
/// axis the pointer has taken further, and a side handle takes the other
/// axis with it, growing away from the top or the left.
pub(crate) fn resized(start: Rect, handle: Handle, target: DVec2, min: DVec2, lock: bool) -> Rect {
    let (x, y) = axes(start, handle);
    let aspect = if start.width > 0.0 && start.height > 0.0 {
        start.width / start.height
    } else {
        1.0
    };
    let width = x.asked(target.x, min.x);
    let height = y.asked(target.y, min.y);
    let width_led = || {
        let across = x.snapped(width, min.x.max(min.y * aspect));
        (across, y.anchored(across.1 / aspect))
    };
    let height_led = || {
        let down = y.snapped(height, min.y.max(min.x / aspect));
        (x.anchored(down.1 * aspect), down)
    };
    let (across, down) = match (x.sign != 0.0, y.sign != 0.0, lock) {
        (false, false, _) => (x.span(), y.span()),
        (true, true, false) => (x.snapped(width, min.x), y.snapped(height, min.y)),
        (true, false, false) => (x.snapped(width, min.x), y.span()),
        (false, true, false) => (x.span(), y.snapped(height, min.y)),
        (true, false, true) => width_led(),
        (false, true, true) => height_led(),
        (true, true, true) => {
            if (width - start.width).abs() >= (height - start.height).abs() {
                width_led()
            } else {
                height_led()
            }
        }
    };
    Rect::new(across.0, down.0, across.1, down.1)
}

/// The bounds of a selection with `handle` dragged to `target`. No grid and
/// no aspect lock: the entities inside scale to whatever this is.
pub(crate) fn resized_bounds(start: Rect, handle: Handle, target: DVec2) -> Rect {
    let (x, y) = axes(start, handle);
    let across = x.anchored(x.asked(target.x, MIN_BOUNDS));
    let down = y.anchored(y.asked(target.y, MIN_BOUNDS));
    Rect::new(across.0, down.0, across.1, down.1)
}

/// Where `entity` lands when the selection bounds `from` become `to`: the
/// same share of the bounds, with every edge on a whole canvas unit so
/// neighbours that touched still touch.
pub(crate) fn placed(entity: Rect, from: Rect, to: Rect) -> Rect {
    let scale = |from: f64, to: f64| if from > 0.0 { to / from } else { 1.0 };
    let (scale_x, scale_y) = (scale(from.width, to.width), scale(from.height, to.height));
    let left = grid::round(to.x + (entity.x - from.x) * scale_x);
    let right = grid::round(to.x + (entity.x + entity.width - from.x) * scale_x);
    let top = grid::round(to.y + (entity.y - from.y) * scale_y);
    let bottom = grid::round(to.y + (entity.y + entity.height - from.y) * scale_y);
    Rect::new(left, top, (right - left).max(1.0), (bottom - top).max(1.0))
}

#[cfg(test)]
mod tests {
    use specular_doc::EdgeSide;

    use super::*;
    use crate::Corner;

    const START: Rect = Rect::new(100.0, 100.0, 400.0, 200.0);
    const MIN: DVec2 = DVec2::new(80.0, 80.0);
    const BOTTOM_RIGHT: Handle = Handle::Corner(Corner::BottomRight);
    const TOP_LEFT: Handle = Handle::Corner(Corner::TopLeft);

    fn at(x: f64, y: f64) -> DVec2 {
        DVec2::new(x, y)
    }

    #[test]
    fn a_corner_moves_two_edges_onto_the_grid() {
        assert_eq!(
            resized(START, BOTTOM_RIGHT, at(547.0, 351.0), MIN, false),
            Rect::new(100.0, 100.0, 440.0, 260.0)
        );
        assert_eq!(
            resized(START, TOP_LEFT, at(53.0, 69.0), MIN, false),
            Rect::new(60.0, 60.0, 440.0, 240.0)
        );

        {
            assert_eq!(
                resized(
                    START,
                    Handle::Side(EdgeSide::Right),
                    at(600.0, 900.0),
                    MIN,
                    false
                ),
                Rect::new(100.0, 100.0, 500.0, 200.0)
            );
            assert_eq!(
                resized(
                    START,
                    Handle::Side(EdgeSide::Top),
                    at(900.0, 40.0),
                    MIN,
                    false
                ),
                Rect::new(100.0, 40.0, 400.0, 260.0)
            );
        }

        {
            assert_eq!(
                resized(START, TOP_LEFT, at(900.0, 900.0), MIN, false),
                Rect::new(420.0, 220.0, 80.0, 80.0)
            );
        }
    }

    #[test]
    fn a_locked_corner_follows_the_axis_dragged_further() {
        // 2:1. The width grows by 100 and the height by 10, so width leads.
        assert_eq!(
            resized(START, BOTTOM_RIGHT, at(600.0, 310.0), MIN, true),
            Rect::new(100.0, 100.0, 500.0, 250.0)
        );
        // The height grows by 100, so it leads and the width follows.
        assert_eq!(
            resized(START, BOTTOM_RIGHT, at(510.0, 400.0), MIN, true),
            Rect::new(100.0, 100.0, 600.0, 300.0)
        );

        {
            assert_eq!(
                resized(START, TOP_LEFT, at(0.0, 95.0), MIN, true),
                Rect::new(0.0, 50.0, 500.0, 250.0)
            );
        }

        {
            assert_eq!(
                resized(
                    START,
                    Handle::Side(EdgeSide::Right),
                    at(600.0, 0.0),
                    MIN,
                    true
                ),
                Rect::new(100.0, 100.0, 500.0, 250.0)
            );
            assert_eq!(
                resized(
                    START,
                    Handle::Side(EdgeSide::Bottom),
                    at(0.0, 400.0),
                    MIN,
                    true
                ),
                Rect::new(100.0, 100.0, 600.0, 300.0)
            );
        }

        {
            // 2:1 with an 80 minimum each way: the width cannot go under 160.
            assert_eq!(
                resized(START, BOTTOM_RIGHT, at(110.0, 110.0), MIN, true),
                Rect::new(100.0, 100.0, 160.0, 80.0)
            );
        }
    }

    #[test]
    fn selection_bounds_follow_the_pointer_exactly_down_to_their_minimum() {
        assert_eq!(
            resized_bounds(START, BOTTOM_RIGHT, at(547.0, 351.0)),
            Rect::new(100.0, 100.0, 447.0, 251.0)
        );
        assert_eq!(
            resized_bounds(START, TOP_LEFT, at(900.0, 900.0)),
            Rect::new(480.0, 280.0, 20.0, 20.0)
        );
        assert_eq!(
            resized_bounds(START, Handle::Side(EdgeSide::Left), at(0.0, 900.0)),
            Rect::new(0.0, 100.0, 500.0, 200.0)
        );
    }

    #[test]
    fn entities_keep_their_share_of_the_bounds_on_whole_units() {
        let from = Rect::new(0.0, 0.0, 300.0, 100.0);
        let to = Rect::new(0.0, 0.0, 450.0, 50.0);
        assert_eq!(
            placed(Rect::new(100.0, 20.0, 101.0, 60.0), from, to),
            Rect::new(150.0, 10.0, 152.0, 30.0)
        );
        // Two rects that shared an edge still share it.
        let left = placed(Rect::new(0.0, 0.0, 101.0, 100.0), from, to);
        let right = placed(Rect::new(101.0, 0.0, 199.0, 100.0), from, to);
        assert_eq!(left.x + left.width, right.x);

        {
            let from = Rect::new(0.0, 0.0, 1000.0, 1000.0);
            let to = Rect::new(0.0, 0.0, 20.0, 20.0);
            assert_eq!(
                placed(Rect::new(500.0, 500.0, 10.0, 10.0), from, to),
                Rect::new(10.0, 10.0, 1.0, 1.0)
            );
        }
    }
}
