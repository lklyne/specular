//! A rounded rectangle whose corners ease into its sides, as the Electron
//! app's chrome has them (`squirclePath.ts`). The Kit rounds a corner with
//! an arc of a circle, so this one is a path to fill.

use gpui_kit::{Bounds, Path, PathBuilder, Pixels, point, px};

/// A quarter circle as a cubic Bézier.
const KAPPA: f32 = 0.552_284_8;
/// How far from a circle's arc the corner is: 0 is the arc, and 0.6 is
/// about what Apple draws.
const SMOOTHING: f32 = 0.6;

/// The outline of `bounds` with corners of `radius`, or `None` when there is
/// no room to draw one.
pub(super) fn squircle(bounds: Bounds<Pixels>, radius: f32) -> Option<Path<Pixels>> {
    let (left, top) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    let most = width.min(height) / 2.0;
    if most <= 0.0 {
        return None;
    }
    let (right, bottom) = (left + width, top + height);
    // Where the curve leaves the side: farther from the corner than an arc
    // of the same radius would.
    let reach = (radius.min(most) * (1.0 + SMOOTHING * 0.3)).min(most);
    let handle = reach * (KAPPA + (1.0 - KAPPA) * SMOOTHING * 0.5);
    let at = |x: f32, y: f32| point(px(x), px(y));

    let mut path = PathBuilder::fill();
    path.move_to(at(left + reach, top));
    path.line_to(at(right - reach, top));
    path.cubic_bezier_to(
        at(right, top + reach),
        at(right - reach + handle, top),
        at(right, top + reach - handle),
    );
    path.line_to(at(right, bottom - reach));
    path.cubic_bezier_to(
        at(right - reach, bottom),
        at(right, bottom - reach + handle),
        at(right - reach + handle, bottom),
    );
    path.line_to(at(left + reach, bottom));
    path.cubic_bezier_to(
        at(left, bottom - reach),
        at(left + reach - handle, bottom),
        at(left, bottom - reach + handle),
    );
    path.line_to(at(left, top + reach));
    path.cubic_bezier_to(
        at(left + reach, top),
        at(left, top + reach - handle),
        at(left + reach - handle, top),
    );
    path.close();
    path.build().ok()
}
