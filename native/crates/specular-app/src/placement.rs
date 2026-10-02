//! Where each page sits on the canvas, and mapping canvas points into it.

use glam::Vec2;
use specular_compositor::PageDraw;
use specular_core::{Camera, CanvasRect, CssSize, PageId};

use crate::paint_lod::PageLod;

/// A page on the canvas: its backend id, canvas rect, CSS viewport and paint
/// LOD. The one record of where a page is, for drawing and hit-testing alike.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlacedPage {
    pub(crate) page: PageId,
    pub(crate) rect: CanvasRect,
    pub(crate) viewport: CssSize,
    pub(crate) lod: PageLod,
}

impl PlacedPage {
    /// A page at `rect` showing a `viewport` layout, at full paint LOD.
    pub(crate) fn new(page: PageId, rect: CanvasRect, viewport: CssSize) -> Self {
        Self {
            page,
            rect,
            viewport,
            lod: PageLod::default(),
        }
    }

    /// What the compositor draws for this page.
    pub(crate) fn draw(&self) -> PageDraw {
        PageDraw {
            page: self.page,
            rect: self.rect,
        }
    }

    /// Screen (logical) pixels per CSS pixel under `camera`, the scale the
    /// paint LOD grades (`pageScreenRect.width / contentSize.width`).
    pub(crate) fn display_scale(&self, camera: &Camera) -> f32 {
        camera.zoom * self.canvas_per_css().x
    }

    /// Converts a canvas (world) point to page-local CSS pixels.
    pub(crate) fn page_local(self, world: Vec2) -> Vec2 {
        let css = Vec2::new(self.viewport.width as f32, self.viewport.height as f32);
        (world - self.rect.origin()) * css / self.rect.size().max(Vec2::splat(f32::EPSILON))
    }

    /// Canvas units per CSS pixel along each axis.
    pub(crate) fn canvas_per_css(self) -> Vec2 {
        let css = Vec2::new(self.viewport.width as f32, self.viewport.height as f32);
        self.rect.size() / css.max(Vec2::ONE)
    }
}

/// The topmost page (last in paint order) containing `world`.
pub(crate) fn hit_test(pages: &[PlacedPage], world: Vec2) -> Option<PlacedPage> {
    pages
        .iter()
        .rev()
        .find(|placed| placed.rect.contains(world))
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(id: u64, rect: CanvasRect, viewport: CssSize) -> PlacedPage {
        PlacedPage::new(PageId(id), rect, viewport)
    }

    #[test]
    fn hit_test_returns_topmost_overlapping_page() {
        let pages = [
            placed(
                1,
                CanvasRect::new(0.0, 0.0, 100.0, 100.0),
                CssSize::new(100, 100),
            ),
            placed(
                2,
                CanvasRect::new(50.0, 50.0, 100.0, 100.0),
                CssSize::new(100, 100),
            ),
        ];
        assert_eq!(
            hit_test(&pages, Vec2::new(75.0, 75.0)).map(|p| p.page),
            Some(PageId(2))
        );
    }

    #[test]
    fn hit_test_misses_empty_canvas() {
        let pages = [placed(
            1,
            CanvasRect::new(0.0, 0.0, 10.0, 10.0),
            CssSize::new(10, 10),
        )];
        assert!(hit_test(&pages, Vec2::new(50.0, 50.0)).is_none());
    }

    #[test]
    fn display_scale_combines_zoom_and_page_scale() {
        // A 1280px page drawn 640 units wide, at zoom 0.5: 0.25 px per CSS px.
        let page = placed(
            1,
            CanvasRect::new(0.0, 0.0, 640.0, 400.0),
            CssSize::new(1280, 800),
        );
        let scale = page.display_scale(&Camera::new(Vec2::ZERO, 0.5));
        assert!((scale - 0.25).abs() < 1e-6);
    }

    #[test]
    fn page_local_goes_negative_left_of_the_page() {
        // A release captured by a page after the pointer left it.
        let page = placed(
            1,
            CanvasRect::new(100.0, 0.0, 1440.0, 900.0),
            CssSize::new(1440, 900),
        );
        assert_eq!(
            page.page_local(Vec2::new(90.0, 10.0)),
            Vec2::new(-10.0, 10.0)
        );
    }

    #[test]
    fn page_local_scales_canvas_rect_to_css_viewport() {
        // A 1280px-wide page shown 640 canvas units wide: 2 CSS px per unit.
        let page = placed(
            1,
            CanvasRect::new(100.0, 100.0, 640.0, 400.0),
            CssSize::new(1280, 800),
        );
        assert_eq!(
            page.page_local(Vec2::new(110.0, 120.0)),
            Vec2::new(20.0, 40.0)
        );
    }
}
