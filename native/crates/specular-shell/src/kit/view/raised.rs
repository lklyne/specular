//! The raised surface: how the chrome shows the one of a set that is on,
//! whether a tab, a tool or a toggle. It is a rim lit along the top and
//! shaded along the bottom, a face lighter than the bar inside it, and a
//! short shadow under it.
//!
//! The Kit can draw neither the rim, a border having one colour, nor the
//! corners, which are squircles. So the surface is painted: one layer under
//! the control's content, the rim's gradient with the face over it,
//! [`RIM`] in from each edge. The shadows are the Kit's own, whose corners
//! are arcs, which their blur hides.

use gpui_kit::{
    BoxShadow, Corners, IntoElement, ParentElement, Styled, canvas, linear_color_stop,
    linear_gradient, point, px, rgba, transparent_black,
};

use super::squircle::squircle;
use crate::theme;

/// The width of the rim.
pub(super) const RIM: f32 = 1.0;

fn layer(drop: f32, blur: f32, spread: f32, color: u32, inset: bool) -> BoxShadow {
    BoxShadow {
        color: theme::tinted(color),
        offset: point(px(0.0), px(drop)),
        blur_radius: px(blur),
        spread_radius: px(spread),
        inset,
    }
}

/// The rim and the face, filling the control they are a child of. The face
/// darkens a little into its bottom edge.
fn surface(radius: f32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let (lit, shaded) = theme::raised_rim();
            if let Some(rim) = squircle(bounds, radius) {
                window.paint_path(
                    rim,
                    linear_gradient(
                        180.0,
                        linear_color_stop(rgba(lit), 0.0),
                        linear_color_stop(rgba(shaded), 1.0),
                    ),
                );
            }
            let face = bounds.dilate(px(-RIM));
            if let Some(fill) = squircle(face, radius - RIM) {
                window.paint_path(fill, theme::solid(theme::raised_fill()));
            }
            window.paint_inset_shadows(
                face,
                Corners::all(px(radius - RIM)),
                &[layer(-2.0, 3.0, -2.0, theme::raised_shade(), true)],
            );
        },
    )
    .absolute()
    .top_0()
    .bottom_0()
    .left_0()
    .right_0()
}

/// A flat squircle of `fill` and corners of `radius`, filling the element
/// it is a child of: what a control that is not on shows under the pointer.
pub(super) fn plate(radius: f32, fill: u32) -> impl IntoElement {
    canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            if let Some(path) = squircle(bounds, radius) {
                window.paint_path(path, theme::solid(fill));
            }
        },
    )
    .size_full()
}

pub(super) trait Raised: Styled + ParentElement + Sized {
    /// Draws this control, whose corners are of `radius`, as the one that
    /// is on. The surface goes under the children added after it, and takes
    /// the place of the control's own fill. The shadow under it is two
    /// short ones, each drawn in from the sides so it shows below the
    /// control and not around it. GPUI takes a blur as the gaussian's
    /// deviation, half of what CSS calls the blur radius.
    fn raised(self, radius: f32) -> Self {
        let (near, far) = theme::raised_shadow();
        self.bg(transparent_black())
            .shadow(vec![
                layer(1.0, 1.0, -1.0, near, false),
                layer(2.0, 2.0, -2.0, far, false),
            ])
            .child(surface(radius))
    }
}

impl<E: Styled + ParentElement> Raised for E {}
