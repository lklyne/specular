//! The part of the viewport the canvas is seen through.
//!
//! The canvas's coordinate system fills the whole window; the built-in
//! toolbar lies across its top and the sidebar over its left edge. What
//! centres, fits, clips or clamps to "the viewport" means what those leave
//! free, as `availableCanvasViewportRect` in `runtime-geometry.ts` and
//! `leftChromeWidth` do in the Electron app.

use glam::{DVec2, Vec2};

use crate::App;
use crate::panel::builtin::TOOLBAR_HEIGHT;

/// A rect of the viewport in logical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Area {
    /// The top-left corner.
    pub min: DVec2,
    /// The size, never negative.
    pub size: DVec2,
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

impl App {
    /// How much of the viewport's left edge a panel covers: the sidebar's
    /// width while it is shown, else nothing. Everything that centres, fits
    /// or clamps to the viewport reads this one answer.
    pub fn covered_left(&self) -> f32 {
        self.session.sidebar.covered_width()
    }
}

/// The part of the viewport the canvas is seen through: right of the
/// sidebar and, with the built-in toolbar on, under it.
pub(crate) fn area(app: &App) -> Area {
    let top = if app.session.panel.built_in {
        f64::from(TOOLBAR_HEIGHT)
    } else {
        0.0
    };
    let min = DVec2::new(f64::from(app.covered_left()), top);
    let size = (app.session.viewport.as_dvec2() - min).max(DVec2::ZERO);
    Area { min, size }
}

/// The point zoom steps hold still: the middle of the width the sidebar
/// leaves free, and of the viewport's height.
pub(crate) fn centre(app: &App) -> Vec2 {
    let left = app.covered_left();
    let viewport = app.session.viewport;
    Vec2::new(left + (viewport.x - left) / 2.0, viewport.y / 2.0)
}
