//! The part of the viewport the canvas is seen through.
//!
//! The canvas's coordinate system fills the whole window; the chrome lies
//! across its top and the sidebar over its left edge. What
//! centres, fits, clips or clamps to "the viewport" means what those leave
//! free, [`App::insets`], as `availableCanvasViewportRect` in
//! `runtime-geometry.ts` and `leftChromeWidth` do in the Electron app.

use glam::{DVec2, Vec2};

use crate::App;

/// A rect of the viewport in logical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Area {
    /// The top-left corner.
    pub(crate) min: DVec2,
    /// The size, never negative.
    pub(crate) size: DVec2,
}

impl Area {
    /// The middle.
    pub(crate) fn centre(self) -> DVec2 {
        self.min + self.size / 2.0
    }

    /// The far corner.
    pub(crate) fn max(self) -> DVec2 {
        self.min + self.size
    }

    /// Whether there is room to show anything.
    pub(crate) fn is_empty(self) -> bool {
        self.size.min_element() <= 0.0
    }
}

/// The part of the viewport the canvas is seen through: inside the
/// insets the regions drawn over it leave.
pub(crate) fn area(app: &App) -> Area {
    let insets = app.insets();
    let min = DVec2::new(f64::from(insets.left), f64::from(insets.top));
    let far = DVec2::new(f64::from(insets.right), f64::from(insets.bottom));
    let size = (app.session.viewport.as_dvec2() - min - far).max(DVec2::ZERO);
    Area { min, size }
}

/// The point zoom steps hold still: the middle of what the canvas is seen
/// through.
pub(crate) fn centre(app: &App) -> Vec2 {
    area(app).centre().as_vec2()
}
