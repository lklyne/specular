//! Size and rect types.
//!
//! - [`Point`], [`Size`] and [`Rect`] are `f32` and carry no unit: what
//!   holds one says whether it is canvas units, logical screen pixels or a
//!   page's CSS pixels.
//! - **CSS space** ([`CssSize`]): a page's layout viewport in CSS pixels
//!   (CEF "DIP" view coordinates). Input is forwarded in this space.
//! - **Pixel space** ([`PixelSize`], [`PixelRect`]): texels of a painted frame,
//!   `css * texture_scale`.

use serde::{Deserialize, Serialize};

/// A page's layout viewport in CSS pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct CssSize {
    /// Width in CSS pixels.
    pub width: u32,
    /// Height in CSS pixels.
    pub height: u32,
}

impl CssSize {
    /// Creates a CSS size.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Pixel size of a frame rastered at `texture_scale` texels per CSS pixel,
    /// rounded up so the frame never under-covers the viewport.
    pub fn to_pixels(self, texture_scale: f32) -> PixelSize {
        let scale = texture_scale.max(0.0);
        PixelSize::new(
            (self.width as f32 * scale).ceil() as u32,
            (self.height as f32 * scale).ceil() as u32,
        )
    }
}

/// Size of a painted frame in texels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PixelSize {
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
}

impl PixelSize {
    /// Creates a pixel size.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Number of texels (`width * height`), widened so large frames cannot overflow.
    pub const fn area(self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Whether either dimension is zero (nothing to draw or upload).
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// An integer rect in a frame's texel space (dirty rects, popup placement).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct PixelRect {
    /// Left edge in texels.
    pub x: i32,
    /// Top edge in texels.
    pub y: i32,
    /// Width in texels.
    pub width: u32,
    /// Height in texels.
    pub height: u32,
}

impl PixelRect {
    /// Creates a pixel rect.
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The rect's size.
    pub const fn size(self) -> PixelSize {
        PixelSize::new(self.width, self.height)
    }
}

/// A position.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// Horizontal position, growing rightwards.
    pub x: f32,
    /// Vertical position, growing downwards.
    pub y: f32,
}

impl Point {
    /// Creates a point.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A width and height.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Size {
    /// Creates a size.
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// An axis-aligned rect.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl Rect {
    /// Creates a rect.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Top-left corner.
    pub const fn origin(self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Width and height.
    pub const fn size(self) -> Size {
        Size::new(self.width, self.height)
    }

    /// The middle of the rect.
    pub fn centre(self) -> Point {
        Point::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    /// Right edge.
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    /// Bottom edge.
    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    /// The rect grown by `by` on every side (shrunk when negative, never
    /// below zero size).
    #[must_use]
    pub fn outset(self, by: f32) -> Self {
        let width = (self.width + by * 2.0).max(0.0);
        let height = (self.height + by * 2.0).max(0.0);
        let centre = self.centre();
        Self::new(
            centre.x - width * 0.5,
            centre.y - height * 0.5,
            width,
            height,
        )
    }

    /// Whether the two rects overlap with positive area.
    pub fn intersects(self, other: Self) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// The overlap of the two rects, or `None` when it has no area.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        (right > x && bottom > y).then(|| Self::new(x, y, right - x, bottom - y))
    }

    /// The smallest rect holding both.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Self::new(x, y, right - x, bottom - y)
    }

    /// The smallest rect holding every point, or `None` for no points.
    pub fn bounding(points: impl IntoIterator<Item = Point>) -> Option<Self> {
        let mut points = points.into_iter();
        let first = points.next()?;
        let (mut min, mut max) = (first, first);
        for point in points {
            min = Point::new(min.x.min(point.x), min.y.min(point.y));
            max = Point::new(max.x.max(point.x), max.y.max(point.y));
        }
        Some(Self::new(min.x, min.y, max.x - min.x, max.y - min.y))
    }
}

/// A rect in a page's CSS pixels, with fractional edges.
pub type CssRect = Rect;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_to_pixels_rounds_up_partial_texels() {
        assert_eq!(CssSize::new(1001, 3).to_pixels(0.5), PixelSize::new(501, 2));
    }

    #[test]
    fn rects_touching_at_an_edge_do_not_intersect() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(10.0, 0.0, 10.0, 10.0);
        assert_eq!((a.intersects(b), a.intersection(b)), (false, None));
    }

    #[test]
    fn outset_grows_every_side_and_stops_at_zero() {
        let rect = Rect::new(10.0, 10.0, 4.0, 8.0);
        assert_eq!(
            (rect.outset(1.0), rect.outset(-3.0)),
            (
                Rect::new(9.0, 9.0, 6.0, 10.0),
                Rect::new(12.0, 13.0, 0.0, 2.0)
            )
        );
    }
}
