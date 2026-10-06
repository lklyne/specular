//! The window, its wgpu surface and the compositor drawing into it.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use glam::Vec2;
use specular_compositor::{
    Compositor, DotGrid, GpuContext, PageDraw, RenderStats, SceneView, ShapeDraw,
};
use specular_core::Camera;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

/// Everything needed to put a frame on screen.
pub(super) struct GpuWindow {
    pub(super) window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    context: GpuContext,
    pub(super) compositor: Compositor,
}

impl GpuWindow {
    pub(super) fn new(
        event_loop: &ActiveEventLoop,
        size: Option<(u32, u32)>,
    ) -> anyhow::Result<Self> {
        let mut attributes = Window::default_attributes().with_title("Specular (Rust spike)");
        if let Some((width, height)) = size {
            attributes = attributes.with_inner_size(LogicalSize::new(width, height));
        }
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .context("creating window")?,
        );
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(Arc::clone(&window))
            .context("creating surface")?;
        let context = pollster::block_on(GpuContext::new(instance, Some(&surface)))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&context.adapter, size.width.max(1), size.height.max(1))
            .context("surface unsupported by adapter")?;
        // Vsync, like Electron's compositor, so frame intervals compare.
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&context.device, &config);
        let compositor =
            Compositor::new(context.device.clone(), context.queue.clone(), config.format);
        tracing::info!(
            adapter = %context.adapter.get_info().name,
            format = ?config.format,
            present_mode = ?config.present_mode,
            "GPU ready"
        );
        Ok(Self {
            window,
            surface,
            config,
            context,
            compositor,
        })
    }

    pub(super) fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width.max(1);
        self.config.height = size.height.max(1);
        self.surface.configure(&self.context.device, &self.config);
    }

    pub(super) fn scale_factor(&self) -> f32 {
        self.window.scale_factor() as f32
    }

    /// Surface size in logical pixels.
    pub(super) fn logical_viewport(&self) -> Vec2 {
        Vec2::new(self.config.width as f32, self.config.height as f32) / self.scale_factor()
    }

    /// The current monitor's refresh interval, when the platform reports it.
    pub(super) fn refresh_interval(&self) -> Option<Duration> {
        let millihertz = self.window.current_monitor()?.refresh_rate_millihertz()?;
        (millihertz > 0).then(|| Duration::from_secs_f64(1_000.0 / f64::from(millihertz)))
    }

    /// Renders and presents one frame; `None` when the surface had no frame
    /// to give (minimised, or reconfigured after loss).
    pub(super) fn render(
        &mut self,
        camera: Camera,
        pages: &[PageDraw],
        shapes: &[ShapeDraw],
    ) -> Option<RenderStats> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.context.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.context.device, &self.config);
                return None;
            }
            _ => return None,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let stats = self.compositor.render(
            &view,
            &SceneView {
                camera,
                viewport: self.logical_viewport(),
                scale_factor: self.scale_factor(),
                pages,
                shapes,
                grid: DotGrid::default(),
            },
        );
        self.window.pre_present_notify();
        self.context.queue.present(frame);
        Some(stats)
    }
}
