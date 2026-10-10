//! The part of the viewport the canvas is seen through.
//!
//! The canvas's coordinate system fills the whole window; the chrome lies
//! across its top and the sidebar over its left edge. What
//! centres, fits, clips or clamps to "the viewport" means what those leave
//! free, as `availableCanvasViewportRect` in `runtime-geometry.ts` and
//! `leftChromeWidth` do in the Electron app.

use glam::{DVec2, Vec2};

use crate::App;
use crate::panel::builtin::DOCK_ROW;

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
    /// How much of the viewport's left edge panels cover: the sidebar's
    /// width while it is shown, and the tools' ribbon while it is docked.
    /// Everything that centres, fits or clamps to the viewport reads this
    /// one answer.
    pub fn covered_left(&self) -> f32 {
        self.session.sidebar.covered_width() + self.tools_dock_width()
    }

    /// How much of the viewport's top edge is covered: the chrome, and
    /// under it the bar a shell that draws its own chrome keeps while a tab
    /// shows its item. On the canvas that bar comes and goes with the
    /// selection, over the canvas, and covers nothing.
    pub fn covered_top(&self) -> f32 {
        let panel = &self.session.panel;
        let bar = if panel.menu_only && self.lens().is_some() {
            DOCK_ROW
        } else {
            0.0
        };
        panel.chrome_height() + bar
    }
}

/// The part of the viewport the canvas is seen through: right of the
/// sidebar and, with the chrome on, under it.
pub(crate) fn area(app: &App) -> Area {
    let top = f64::from(app.covered_top());
    let min = DVec2::new(f64::from(app.covered_left()), top);
    let size = (app.session.viewport.as_dvec2() - min).max(DVec2::ZERO);
    Area { min, size }
}

/// The point zoom steps hold still: the middle of what the canvas is seen
/// through.
pub(crate) fn centre(app: &App) -> Vec2 {
    area(app).centre().as_vec2()
}
