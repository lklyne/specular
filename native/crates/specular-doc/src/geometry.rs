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

    /// Whether `point` is inside: the left and top edges count, the right
    /// and bottom do not, so rects that share an edge never both hold it.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }

    /// The smallest rect holding both.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let (x, y) = (self.x.min(other.x), self.y.min(other.y));
        let right = (self.x + self.width).max(other.x + other.width);
        let bottom = (self.y + self.height).max(other.y + other.height);
        Self::new(x, y, right - x, bottom - y)
    }

    /// The part of `self` that `other` covers, or `None` when they share no
    /// area.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let (x, y) = (self.x.max(other.x), self.y.max(other.y));
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > x && bottom > y).then(|| Self::new(x, y, right - x, bottom - y))
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
