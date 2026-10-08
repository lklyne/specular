//! Size and rect types for the three coordinate spaces the spike crosses.
//!
//! - **Canvas space** ([`CanvasRect`]): world units on the infinite canvas,
//!   the space `.canvas` node `x/y/width/height` are stored in.
//! - **CSS space** ([`CssSize`]): a page's layout viewport in CSS pixels
//!   (CEF "DIP" view coordinates). Input is forwarded in this space.
//! - **Pixel space** ([`PixelSize`], [`PixelRect`]): texels of a painted frame,
//!   `css * texture_scale`.

use glam::Vec2;
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

/// An axis-aligned rect in a page's CSS pixels, with fractional edges.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CssRect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

impl CssRect {
    /// Creates a CSS-pixel rect.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// An axis-aligned rect in canvas (world) space.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct CanvasRect {
    /// Left edge in canvas units.
    pub x: f32,
    /// Top edge in canvas units.
    pub y: f32,
    /// Width in canvas units.
    pub width: f32,
    /// Height in canvas units.
    pub height: f32,
}

impl CanvasRect {
    /// Creates a canvas rect.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Top-left corner.
    pub const fn origin(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    /// Width and height as a vector.
    pub const fn size(self) -> Vec2 {
        Vec2::new(self.width, self.height)
    }

    /// Whether `point` lies inside the rect (left/top inclusive, right/bottom exclusive).
    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }

    /// Whether the two rects overlap with positive area.
    pub fn intersects(self, other: CanvasRect) -> bool {
        self.x < other.x + other.width
            && other.x < self.x + self.width
            && self.y < other.y + other.height
            && other.y < self.y + self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_to_pixels_rounds_up_partial_texels() {
        assert_eq!(CssSize::new(1001, 3).to_pixels(0.5), PixelSize::new(501, 2));
    }
}
