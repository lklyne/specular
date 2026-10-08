//! [`PagePlacement`]: where a page sits on the canvas, and mapping canvas
//! points into its CSS pixels.

use glam::DVec2;
use specular_core::{Camera, CssSize};
use specular_doc::Rect;

use crate::geometry;

/// A page's entity rect and the CSS viewport its content is laid out at.
/// The two differ only while the page is being resized.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PagePlacement {
    /// The entity rect in canvas space.
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
    pub fn canvas_per_css(self) -> DVec2 {
        geometry::size(self.rect) / self.css().max(DVec2::ONE)
    }

    /// A canvas point in the page's CSS pixels. Negative or past the
    /// viewport when the point is outside the page.
    pub fn page_local(self, world: DVec2) -> DVec2 {
        let size = geometry::size(self.rect).max(DVec2::splat(f64::EPSILON));
        (world - geometry::origin(self.rect)) * self.css() / size
    }

    /// A point in the page's CSS pixels as a canvas point.
    pub fn to_canvas(self, css: DVec2) -> DVec2 {
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

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use super::*;

    fn placed(rect: Rect, viewport: CssSize) -> PagePlacement {
        PagePlacement { rect, viewport }
    }

    #[test]
    fn viewport_is_the_rounded_rect_size_and_never_empty() {
        let viewport = PagePlacement::viewport_for(Rect::new(0.0, 0.0, 519.6, 0.2));
        assert_eq!(viewport, CssSize::new(520, 1));
    }

    #[test]
    fn display_scale_combines_zoom_and_page_scale() {
        // A 1280px page drawn 640 units wide, at zoom 0.5: 0.25 px per CSS px.
        let page = placed(Rect::new(0.0, 0.0, 640.0, 400.0), CssSize::new(1280, 800));
        let scale = page.display_scale(&Camera::new(Vec2::ZERO, 0.5));
        assert!((scale - 0.25).abs() < 1e-6);
    }

    #[test]
    fn page_local_scales_canvas_rect_to_css_viewport() {
        // A 1280px-wide page shown 640 canvas units wide: 2 CSS px per unit.
        let page = placed(
            Rect::new(100.0, 100.0, 640.0, 400.0),
            CssSize::new(1280, 800),
        );
        assert_eq!(
            page.page_local(DVec2::new(110.0, 120.0)),
            DVec2::new(20.0, 40.0)
        );

        {
            // A release captured by a page after the pointer left it.
            let page = placed(
                Rect::new(100.0, 0.0, 1440.0, 900.0),
                CssSize::new(1440, 900),
            );
            assert_eq!(
                page.page_local(DVec2::new(90.0, 10.0)),
                DVec2::new(-10.0, 10.0)
            );
        }

        {
            let page = placed(
                Rect::new(100.0, 100.0, 640.0, 400.0),
                CssSize::new(1280, 800),
            );
            assert_eq!(
                page.to_canvas(DVec2::new(20.0, 40.0)),
                DVec2::new(110.0, 120.0)
            );
        }
    }
}
