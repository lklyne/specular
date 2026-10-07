//! The offscreen harness shared by the `render_scene` GPU tests.

use glam::Vec2;
use specular_compositor::{Compositor, DotGrid, FrameView, GpuContext, SceneStats};
use specular_core::{Camera, PageId};
use specular_scene::{Item, Scene};

use crate::common::{TARGET_SIZE, gpu_or_skip, read_pixels, render_target};

/// The page host behind the entity id `"page"`.
pub(crate) const PAGE: PageId = PageId(1);
/// Linear blue; encodes to exactly (0, 0, 255).
pub(crate) const BACKGROUND: [u8; 4] = [0, 0, 255, 255];

pub(crate) struct Harness {
    pub(crate) gpu: GpuContext,
    pub(crate) compositor: Compositor,
    pub(crate) target: wgpu::Texture,
}

impl Harness {
    /// A compositor drawing into an `Rgba8Unorm` target.
    pub(crate) fn new() -> Option<Self> {
        Self::with_format(wgpu::TextureFormat::Rgba8Unorm)
    }

    pub(crate) fn with_format(format: wgpu::TextureFormat) -> Option<Self> {
        let gpu = gpu_or_skip()?;
        let compositor = Compositor::new(gpu.device.clone(), gpu.queue.clone(), format);
        let target = render_target(&gpu, format);
        Some(Self {
            gpu,
            compositor,
            target,
        })
    }

    /// Draws `items` for `frame` and reads the target back.
    pub(crate) fn render_frame(
        &mut self,
        frame: &FrameView,
        items: Vec<Item>,
    ) -> (Vec<[u8; 4]>, SceneStats) {
        let view = self
            .target
            .create_view(&wgpu::TextureViewDescriptor::default());
        let stats = self
            .compositor
            .render_scene(&view, frame, &Scene { items }, |entity| {
                (entity.as_str() == "page").then_some(PAGE)
            });
        (read_pixels(&self.gpu, &self.target), stats)
    }

    /// Draws `items` under the default camera at one pixel per unit.
    pub(crate) fn render(&mut self, items: Vec<Item>) -> Vec<[u8; 4]> {
        self.render_frame(&frame(Camera::default()), items).0
    }
}

/// A still frame over a plain blue background at scale factor 1.
pub(crate) fn frame(camera: Camera) -> FrameView {
    FrameView {
        camera,
        viewport: Vec2::splat(TARGET_SIZE as f32),
        scale_factor: 1.0,
        grid: DotGrid {
            spacing: 0.0,
            background: [0.0, 0.0, 1.0, 1.0],
            ..DotGrid::default()
        },
        zooming: false,
    }
}
