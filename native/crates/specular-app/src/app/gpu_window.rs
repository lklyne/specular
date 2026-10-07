//! The window, its wgpu surface and the compositor drawing into it.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use glam::Vec2;
use specular_compositor::{Compositor, DotGrid, FrameView, GpuContext, SceneStats};
use specular_core::{Camera, PageId};
use specular_doc::EntityId;
use specular_scene::Scene;
use winit::dpi::{LogicalSize, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowLevel};

/// Everything needed to put a frame on screen.
pub(super) struct GpuWindow {
    pub(super) window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    context: GpuContext,
    pub(super) compositor: Compositor,
    /// Whether the last render presented, so a change is logged once.
    presenting: bool,
}

impl GpuWindow {
    pub(super) fn new(
        event_loop: &ActiveEventLoop,
        size: Option<(u32, u32)>,
        always_on_top: bool,
    ) -> anyhow::Result<Self> {
        let mut attributes = Window::default_attributes().with_title("Specular (Rust spike)");
        if always_on_top {
            // macOS stops presenting a fully covered window, which a bench
            // run would record as phases with no frames.
            attributes = attributes.with_window_level(WindowLevel::AlwaysOnTop);
        }
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
        let mut compositor =
            Compositor::new(context.device.clone(), context.queue.clone(), config.format);
        // Loading the system fonts takes a moment. Better here than on the
        // first frame that shows text.
        compositor.warm_text();
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
            presenting: true,
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

    /// Logs when frames stop or resume reaching the screen. A bench phase run
    /// while nothing presents records no frames, so the log has to say why.
    fn note_presenting(&mut self, presenting: bool, reason: &str) {
        if presenting == self.presenting {
            return;
        }
        self.presenting = presenting;
        if presenting {
            tracing::info!("presenting frames again");
        } else {
            tracing::warn!(reason, "not presenting frames");
        }
    }

    /// Renders and presents one frame; `None` when the surface had no frame
    /// to give (minimised, or reconfigured after loss).
    pub(super) fn render(
        &mut self,
        camera: Camera,
        zooming: bool,
        scene: &Scene,
        page_of: impl Fn(&EntityId) -> Option<PageId>,
    ) -> Option<SceneStats> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.context.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.context.device, &self.config);
                self.note_presenting(false, "surface outdated or lost");
                return None;
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                self.note_presenting(false, "window occluded");
                return None;
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                self.note_presenting(false, "surface timed out");
                return None;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                self.note_presenting(false, "surface validation error");
                return None;
            }
        };
        self.note_presenting(true, "");
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let frame_view = FrameView {
            camera,
            viewport: self.logical_viewport(),
            scale_factor: self.scale_factor(),
            grid: DotGrid::default(),
            zooming,
        };
        let stats = self
            .compositor
            .render_scene(&view, &frame_view, scene, page_of);
        self.window.pre_present_notify();
        self.context.queue.present(frame);
        Some(stats)
    }
}
