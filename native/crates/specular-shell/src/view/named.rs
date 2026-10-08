//! Where each model control is on screen, by the model's name for it.
//!
//! Every element that draws a model control holds a [`mark`]: a child that
//! paints nothing and notes its parent's bounds as GPUI lays the frame out.
//! A script then clicks `shape.color` wherever the Kit put it, as the
//! headless runner clicks it wherever the built-in panels do. The names are
//! the models' own, so a dropdown's trigger is the dropdown's name and a
//! stepper's two buttons are its `.dec` and `.inc`.
//!
//! The list is of the frame GPUI last laid out. A control scrolled out of
//! its list is not in it: a click at its centre would land on something
//! else.

use std::cell::RefCell;

use glam::Vec2;
use gpui_kit::{Bounds, IntoElement, Pixels, Styled as _, canvas};
use specular_interact::ControlId;

thread_local! {
    /// The controls of the frame being laid out, in paint order.
    static SHOWN: RefCell<Vec<(String, Vec2)>> = const { RefCell::new(Vec::new()) };
}

/// Forgets the last frame's controls. The root view calls it as it renders.
pub(super) fn begin_frame() {
    SHOWN.with(|shown| shown.borrow_mut().clear());
}

fn centre(bounds: Bounds<Pixels>) -> Vec2 {
    let centre = bounds.center();
    Vec2::new(f32::from(centre.x), f32::from(centre.y))
}

/// An element that notes where the control named `id` is: the bounds of
/// what it is a child of.
pub(super) fn mark(id: &ControlId) -> impl IntoElement + use<> {
    let name = id.as_str().to_owned();
    canvas(
        move |bounds, window, _| {
            // What an enclosing scroll list cuts off cannot be clicked.
            if window.content_mask().bounds.contains(&bounds.center()) {
                SHOWN.with(|shown| shown.borrow_mut().push((name, centre(bounds))));
            }
        },
        |_, (), _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// The names of the Kit's controls on screen and the middle of each, in
/// points from the window's content corner, in paint order.
pub(crate) fn shown() -> Vec<(String, Vec2)> {
    SHOWN.with(|shown| shown.borrow().clone())
}
