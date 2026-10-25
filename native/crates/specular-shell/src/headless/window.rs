//! The window a headless run draws into: an offscreen texture on the real
//! adapter. It has no cursor, input method or title, and a frame is done
//! when the GPU is.

use anyhow::Context as _;
use glam::Vec2;
use specular_compositor::{Compositor, DotGrid, FrameView, GpuContext, SceneStats};
use specular_core::Camera;
use specular_interact::{ControlId, Cursor};
use specular_scene::Scene;

use crate::app::{PageOf, ShellWindow};
use crate::offscreen::{self, Target};

/// An offscreen texture and the compositor that draws into it.
#[derive(Debug)]
pub(crate) struct HeadlessWindow {
    gpu: GpuContext,
    compositor: Compositor,
    target: Target,
    viewport: Vec2,
    scale: f32,
}

impl HeadlessWindow {
    /// A window of `size` logical pixels at `scale` device pixels each.
    pub(crate) fn new((width, height): (u32, u32), scale: f32) -> anyhow::Result<Self> {
        let gpu = pollster::block_on(GpuContext::headless()).context("no GPU to draw with")?;
        let mut compositor =
            Compositor::new(gpu.device.clone(), gpu.queue.clone(), offscreen::FORMAT);
        compositor.warm_text();
        let pixels = |side: u32| ((side as f32 * scale).round() as u32).max(1);
        let target = Target::new(&gpu, pixels(width), pixels(height), offscreen::FORMAT);
        tracing::info!(adapter = %gpu.adapter.get_info().name, "drawing headless");
        Ok(Self {
            gpu,
            compositor,
            target,
            viewport: Vec2::new(width as f32, height as f32),
            scale,
        })
    }

    /// Writes the last frame drawn to `path` as a PNG.
    pub(crate) fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        self.target.save(&self.gpu, path)
    }

    fn capture_sized(
        &mut self,
        camera: Camera,
        viewport: Vec2,
        (width, height): (u32, u32),
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        let target = Target::new(&self.gpu, width, height, offscreen::FORMAT);
        let frame = FrameView {
            camera,
            viewport,
            scale_factor: self.scale,
            grid: DotGrid::themed(scene.colors()),
            zooming: false,
        };
        self.compositor
            .render_scene(&target.view(), &frame, scene, page_of);
        Ok((target.png(&self.gpu)?, width, height))
    }
}

impl ShellWindow for HeadlessWindow {
    fn compositor(&self) -> &Compositor {
        &self.compositor
    }

    fn compositor_mut(&mut self) -> &mut Compositor {
        &mut self.compositor
    }

    fn scale_factor(&self) -> f32 {
        self.scale
    }

    fn logical_viewport(&self) -> Vec2 {
        self.viewport
    }

    fn capture_area(
        &mut self,
        camera: Camera,
        viewport: Vec2,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        let pixels = |side: f32| ((side * self.scale).round() as u32).max(1);
        let size = (pixels(viewport.x), pixels(viewport.y));
        self.capture_sized(camera, viewport, size, scene, page_of)
    }

    fn render(
        &mut self,
        camera: Camera,
        zooming: bool,
        scene: &mut Scene,
        page_of: PageOf<'_>,
    ) -> Option<SceneStats> {
        let frame = FrameView {
            camera,
            viewport: self.viewport,
            scale_factor: self.scale,
            grid: DotGrid::themed(scene.colors()),
            zooming,
        };
        let stats = (self.compositor).render_scene(&self.target.view(), &frame, scene, page_of);
        // Done when the GPU is: there is no display to wait for.
        if let Err(error) = self.gpu.device.poll(wgpu::PollType::wait_indefinitely()) {
            tracing::warn!("waiting for the frame: {error}");
        }
        Some(stats)
    }

    fn capture(
        &mut self,
        camera: Camera,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        let size = self.target.size();
        self.capture_sized(camera, self.viewport, size, scene, page_of)
    }

    fn set_ime_allowed(&self, _: bool) {}

    fn set_ime_cursor_area(&self, _: Vec2, _: Vec2) {}

    fn set_cursor(&self, _: Cursor) {}

    fn edit_field(&self, _: &ControlId) {}

    fn set_title(&self, _: &str, _: bool) {}
}
