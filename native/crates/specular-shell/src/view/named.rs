//! Where each model control is on screen, by the model's name for it.
//!
//! Every element that draws a model control holds a [`mark`]: a child that
//! paints nothing and notes its parent's bounds as GPUI lays the frame out.
//! A script then clicks `shape.color` wherever the Kit put it, as the
//! headless runner clicks it wherever the built-in panels do. The names are
//! the models' own, so a dropdown's trigger is the dropdown's name and a
//! stepper's two buttons are its `.dec` and `.inc`.
//!
//! The list is of what GPUI last laid out. Each view that draws controls
//! keeps its own, replaced when the view is built again: a cached view is
//! not laid out on a frame that reuses it, and its controls are where they
//! were. A control scrolled out of its list is not in it: a click at its
//! centre would land on something else.

use std::cell::{Cell, RefCell};

use glam::Vec2;
use gpui_kit::{Bounds, IntoElement, Pixels, Styled as _, canvas};
use specular_interact::ControlId;

use crate::ui_stats::Part;

/// The views that draw named controls, in paint order.
const PARTS: [Part; 3] = [Part::Sidebar, Part::Toolbar, Part::Popup];

thread_local! {
    /// Each part's controls as it last laid them out, in paint order.
    static SHOWN: RefCell<[Vec<(String, Vec2)>; PARTS.len()]> =
        const { RefCell::new([Vec::new(), Vec::new(), Vec::new()]) };
    /// The part whose elements are being made.
    static BUILDING: Cell<Option<usize>> = const { Cell::new(None) };
}

fn slot(part: Part) -> Option<usize> {
    PARTS.iter().position(|known| *known == part)
}

/// `part` is being built again: its controls are forgotten, and the marks
/// made from here on are its own. Each view that draws named controls calls
/// this as it renders.
pub(super) fn begin(part: Part) {
    forget(part);
    BUILDING.set(slot(part));
}

/// `part` is not drawn, so none of its controls is on screen.
pub(super) fn forget(part: Part) {
    if let Some(slot) = slot(part) {
        SHOWN.with(|shown| shown.borrow_mut()[slot].clear());
    }
}

fn centre(bounds: Bounds<Pixels>) -> Vec2 {
    let centre = bounds.center();
    Vec2::new(f32::from(centre.x), f32::from(centre.y))
}

/// An element that notes where the control named `id` is: the bounds of
/// what it is a child of.
pub(super) fn mark(id: &ControlId) -> impl IntoElement + use<> {
    let name = id.as_str().to_owned();
    let part = BUILDING.get();
    canvas(
        move |bounds, window, _| {
            // What an enclosing scroll list cuts off cannot be clicked.
            let Some(part) =
                part.filter(|_| window.content_mask().bounds.contains(&bounds.center()))
            else {
                return;
            };
            SHOWN.with(|shown| shown.borrow_mut()[part].push((name, centre(bounds))));
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
    SHOWN.with(|shown| shown.borrow().iter().flatten().cloned().collect())
}
