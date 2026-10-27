//! [`PagePlacement`]: where a page sits on the canvas, and mapping canvas
//! points into its CSS pixels.

use glam::DVec2;
use specular_core::{Camera, CssSize};
use specular_doc::Rect;

use crate::geometry;

/// Where a page's document is drawn on the canvas and the CSS viewport it
/// is laid out at there. The rect is the entity's, less the safe area of
/// the device a framed page is shown as.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PagePlacement {
    /// Where the document is drawn, in canvas space.
    pub rect: Rect,
    /// The layout viewport in CSS pixels.
    pub viewport: CssSize,
}

impl PagePlacement {
    /// The viewport a page with this rect is laid out at: the rect's size in
    /// whole CSS pixels, at least one each way.
    pub fn viewport_for(rect: Rect) -> CssSize {
        let size = geometry::size(rect).round().max(DVec2::ONE);
        CssSize::new(size.x as u32, size.y as u32)
    }

    fn css(self) -> DVec2 {
        DVec2::new(
            f64::from(self.viewport.width),
            f64::from(self.viewport.height),
        )
    }

    /// Canvas units per CSS pixel along each axis.
    pub(crate) fn canvas_per_css(self) -> DVec2 {
        geometry::size(self.rect) / self.css().max(DVec2::ONE)
    }

    /// A canvas point in the page's CSS pixels. Negative or past the
    /// viewport when the point is outside the page.
    pub(crate) fn page_local(self, world: DVec2) -> DVec2 {
        let size = geometry::size(self.rect).max(DVec2::splat(f64::EPSILON));
        (world - geometry::origin(self.rect)) * self.css() / size
    }

    /// A point in the page's CSS pixels as a canvas point.
    pub(crate) fn to_canvas(self, css: DVec2) -> DVec2 {
        geometry::origin(self.rect) + css * self.canvas_per_css()
    }

    /// The rect's top-left corner in the `f32` canvas space the camera
    /// projects.
    pub(crate) fn rect_origin(self) -> glam::Vec2 {
        geometry::origin(self.rect).as_vec2()
    }

    /// Logical screen pixels per CSS pixel under `camera`: the scale the
    /// paint LOD grades.
    pub fn display_scale(self, camera: &Camera) -> f32 {
        camera.zoom * self.canvas_per_css().x as f32
    }
}
