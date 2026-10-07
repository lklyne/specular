//! Canvas-space geometry, in `f64` because `.canvas` numbers are JSON
//! doubles and must survive a load and save unchanged.

use serde::{Deserialize, Serialize};

/// A point in canvas space.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Point {
    /// Horizontal position in canvas units.
    pub x: f64,
    /// Vertical position in canvas units.
    pub y: f64,
}

impl Point {
    /// Creates a point.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rect: an entity rect in canvas space, or a region in a
/// page's document space.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Rect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

impl Rect {
    /// Creates a rect.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The same rect moved by `(dx, dy)`.
    #[must_use]
    pub fn translated(self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }
}
