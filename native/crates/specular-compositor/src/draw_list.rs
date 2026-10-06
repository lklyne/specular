//! Turns a [`SceneView`] into the quads to draw this frame.
//!
//! Pure: it only needs to know which layers each page has, so culling, popup
//! placement and the stats counters are testable without a GPU.

use glam::Vec2;
use specular_core::{PageId, PixelRect, PixelSize};

use crate::gpu_types::QuadInstance;
use crate::scene::SceneView;

/// Page corner radius in canvas units (canvas-bg's page chrome rounding).
pub(crate) const PAGE_CORNER_RADIUS: f32 = 8.0;

/// Which of a page's textures a quad samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LayerKind {
    /// The page's main view.
    View,
    /// The popup widget drawn over it.
    Popup,
}

/// One draw call: quad `instance` of the instance buffer, textured by
/// `page`'s `layer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DrawItem {
    pub(crate) page: PageId,
    pub(crate) layer: LayerKind,
}

/// What the compositor currently holds for a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PageLayersInfo {
    /// Size of the view texture, in texels.
    pub(crate) view_size: PixelSize,
    /// Whether the view texture came from a CPU upload.
    pub(crate) view_is_cpu: bool,
    /// Placement of a visible, painted popup in view texels.
    pub(crate) popup: Option<PixelRect>,
}

/// Counters produced while building the draw list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DrawCounts {
    pub(crate) pages_without_texture: u32,
    pub(crate) cpu_textures: u32,
}

/// Fills `instances`/`items` (cleared first, capacity reused) with the quads
/// for `scene`, skipping pages outside the viewport.
pub(crate) fn build_draw_list(
    scene: &SceneView<'_>,
    layers: impl Fn(PageId) -> Option<PageLayersInfo>,
    instances: &mut Vec<QuadInstance>,
    items: &mut Vec<DrawItem>,
) -> DrawCounts {
    instances.clear();
    items.clear();
    let visible = scene.camera.visible_world_rect(scene.viewport);
    let mut counts = DrawCounts::default();
    for draw in scene.pages {
        if !draw.rect.intersects(visible) {
            continue;
        }
        let Some(info) = layers(draw.page) else {
            counts.pages_without_texture += 1;
            continue;
        };
        counts.cpu_textures += u32::from(info.view_is_cpu);
        instances.push(QuadInstance::new(
            draw.rect.origin(),
            draw.rect.size(),
            PAGE_CORNER_RADIUS,
        ));
        items.push(DrawItem {
            page: draw.page,
            layer: LayerKind::View,
        });
        if let Some(popup) = info.popup
            && !info.view_size.is_empty()
        {
            let texel_to_canvas = draw.rect.size()
                / Vec2::new(info.view_size.width as f32, info.view_size.height as f32);
            let origin =
                draw.rect.origin() + Vec2::new(popup.x as f32, popup.y as f32) * texel_to_canvas;
            let size = Vec2::new(popup.width as f32, popup.height as f32) * texel_to_canvas;
            instances.push(QuadInstance::new(origin, size, 0.0));
            items.push(DrawItem {
                page: draw.page,
                layer: LayerKind::Popup,
            });
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use specular_core::{Camera, CanvasRect};

    use super::*;
    use crate::scene::{DotGrid, PageDraw};

    fn scene(pages: &[PageDraw]) -> SceneView<'_> {
        SceneView {
            camera: Camera::default(),
            viewport: Vec2::new(800.0, 600.0),
            scale_factor: 1.0,
            pages,
            shapes: &[],
            grid: DotGrid::default(),
        }
    }

    fn page(id: u64, x: f32) -> PageDraw {
        PageDraw {
            page: PageId(id),
            rect: CanvasRect::new(x, 0.0, 200.0, 100.0),
        }
    }

    fn view_only(size: PixelSize) -> PageLayersInfo {
        PageLayersInfo {
            view_size: size,
            view_is_cpu: false,
            popup: None,
        }
    }

    fn build(
        pages: &[PageDraw],
        layers: impl Fn(PageId) -> Option<PageLayersInfo>,
    ) -> (Vec<QuadInstance>, Vec<DrawItem>, DrawCounts) {
        let (mut instances, mut items) = (Vec::new(), Vec::new());
        let counts = build_draw_list(&scene(pages), layers, &mut instances, &mut items);
        (instances, items, counts)
    }

    #[test]
    fn pages_outside_viewport_are_culled() {
        let pages = [page(1, 0.0), page(2, 5_000.0)];
        let (_, items, _) = build(&pages, |_| Some(view_only(PixelSize::new(200, 100))));
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn visible_page_without_texture_is_counted_not_drawn() {
        let (_, items, counts) = build(&[page(1, 0.0)], |_| None);
        assert_eq!((items.len(), counts.pages_without_texture), (0, 1));
    }

    #[test]
    fn cpu_textured_pages_are_counted() {
        let info = PageLayersInfo {
            view_is_cpu: true,
            ..view_only(PixelSize::new(200, 100))
        };
        let (_, _, counts) = build(&[page(1, 0.0), page(2, 300.0)], |_| Some(info));
        assert_eq!(counts.cpu_textures, 2);
    }

    #[test]
    fn popup_is_drawn_after_its_view_in_canvas_space() {
        // View texture is 2x the canvas rect, so popup texels halve.
        let info = PageLayersInfo {
            popup: Some(PixelRect::new(40, 20, 100, 60)),
            ..view_only(PixelSize::new(400, 200))
        };
        let (instances, items, _) = build(&[page(1, 10.0)], |_| Some(info));
        assert_eq!(items[1].layer, LayerKind::Popup);
        let rect = glam::Vec4::from_array(instances[1].rect);
        assert!(rect.abs_diff_eq(glam::Vec4::new(30.0, 10.0, 50.0, 30.0), 1e-5));
    }

    #[test]
    fn draw_list_preserves_paint_order() {
        let pages = [page(3, 0.0), page(1, 100.0)];
        let (_, items, _) = build(&pages, |_| Some(view_only(PixelSize::new(1, 1))));
        let order: Vec<_> = items.iter().map(|item| item.page).collect();
        assert_eq!(order, [PageId(3), PageId(1)]);
    }
}
