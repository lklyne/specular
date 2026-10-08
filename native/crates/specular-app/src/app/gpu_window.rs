//! The window, its wgpu surface and the compositor drawing into it.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context as _;
use glam::Vec2;
use specular_compositor::{Compositor, DotGrid, FrameView, GpuContext, SceneStats};
use specular_core::{Camera, PageId};
use specular_doc::EntityId;
use specular_interact::Cursor;
use specular_scene::Scene;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowLevel};

use super::runtime::{PageOf, ShellWindow};
use crate::offscreen::Target;
use crate::translate;

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
        // Not an sRGB format: the compositor then blends encoded colours, as
        // a browser does. In linear light dark text on a light ground comes
        // out thin and grey, and a see-through fill over it far too bright.
        let plain = config.format.remove_srgb_suffix();
        if surface
            .get_capabilities(&context.adapter)
            .formats
            .contains(&plain)
        {
            config.format = plain;
        }
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
    /// Draws `scene` as the window would show it into a texture, and
    /// returns the PNG with its size in device pixels. The window itself is
    /// not touched, so this works while it is covered or minimised.
    pub(super) fn capture(
        &mut self,
        camera: Camera,
        scene: &Scene,
        page_of: impl Fn(&EntityId) -> Option<PageId>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        let (viewport, size) = (
            self.logical_viewport(),
            (self.config.width, self.config.height),
        );
        self.capture_sized(camera, viewport, size, scene, page_of)
    }

    /// Draws `scene` into a texture of `viewport` logical pixels, whatever
    /// size the window is.
    pub(super) fn capture_area(
        &mut self,
        camera: Camera,
        viewport: Vec2,
        scene: &Scene,
        page_of: impl Fn(&EntityId) -> Option<PageId>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        let pixels = |side: f32| ((side * self.scale_factor()).round() as u32).max(1);
        let size = (pixels(viewport.x), pixels(viewport.y));
        self.capture_sized(camera, viewport, size, scene, page_of)
    }

    fn capture_sized(
        &mut self,
        camera: Camera,
        viewport: Vec2,
        (width, height): (u32, u32),
        scene: &Scene,
        page_of: impl Fn(&EntityId) -> Option<PageId>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        let target = Target::new(&self.context, width, height, self.config.format);
        let frame_view = FrameView {
            camera,
            viewport,
            scale_factor: self.scale_factor(),
            grid: DotGrid::default(),
            zooming: false,
        };
        self.compositor
            .render_scene(&target.view(), &frame_view, scene, page_of);
        let (width, height) = target.size();
        Ok((target.png(&self.context)?, width, height))
    }

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

impl ShellWindow for GpuWindow {
    fn compositor(&self) -> &Compositor {
        &self.compositor
    }

    fn compositor_mut(&mut self) -> &mut Compositor {
        &mut self.compositor
    }

    fn scale_factor(&self) -> f32 {
        Self::scale_factor(self)
    }

    fn logical_viewport(&self) -> Vec2 {
        Self::logical_viewport(self)
    }

    fn render(
        &mut self,
        camera: Camera,
        zooming: bool,
        scene: &mut Scene,
        page_of: PageOf<'_>,
    ) -> Option<SceneStats> {
        Self::render(self, camera, zooming, scene, page_of)
    }

    fn capture(
        &mut self,
        camera: Camera,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        Self::capture(self, camera, scene, page_of)
    }

    fn capture_area(
        &mut self,
        camera: Camera,
        viewport: Vec2,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)> {
        Self::capture_area(self, camera, viewport, scene, page_of)
    }

    fn set_ime_allowed(&self, allowed: bool) {
        self.window.set_ime_allowed(allowed);
    }

    fn set_ime_cursor_area(&self, origin: Vec2, size: Vec2) {
        self.window.set_ime_cursor_area(
            LogicalPosition::new(origin.x, origin.y),
            LogicalSize::new(size.x, size.y),
        );
    }

    fn set_cursor(&self, cursor: Cursor) {
        self.window.set_cursor(translate::cursor_icon(cursor));
    }

    fn set_title(&self, title: &str, unsaved: bool) {
        self.window.set_title(title);
        #[cfg(target_os = "macos")]
        winit::platform::macos::WindowExtMacOS::set_document_edited(&*self.window, unsaved);
        #[cfg(not(target_os = "macos"))]
        let _ = unsaved;
    }
}
