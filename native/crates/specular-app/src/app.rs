//! The winit application: one window, one compositor, one page source.

use std::sync::Arc;

use anyhow::Context as _;
use glam::Vec2;
use specular_compositor::{Compositor, DotGrid, GpuContext, PageDraw, SceneView};
use specular_core::document::PageNode;
use specular_core::{Camera, CssSize, PageEvent, PageSource, PageSpec, ViewportInputDelta};
use winit::application::ApplicationHandler;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

/// Logical pixels per wheel "line" for mice that report line deltas.
const PIXELS_PER_LINE: f32 = 40.0;

struct Gpu {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    context: GpuContext,
    compositor: Compositor,
}

/// Application state driven by winit.
pub(crate) struct App {
    source: Box<dyn PageSource>,
    initial_pages: Vec<PageNode>,
    draws: Vec<PageDraw>,
    camera: Camera,
    gpu: Option<Gpu>,
    events: Vec<PageEvent>,
    zoom_modifier: bool,
    error: Option<anyhow::Error>,
}

impl App {
    pub(crate) fn new(source: Box<dyn PageSource>, initial_pages: Vec<PageNode>) -> Self {
        Self {
            source,
            initial_pages,
            draws: Vec::new(),
            camera: Camera::new(Vec2::new(40.0, 40.0), 0.25),
            gpu: None,
            events: Vec::new(),
            zoom_modifier: false,
            error: None,
        }
    }

    /// The first fatal error hit inside the event loop, if any.
    pub(crate) fn into_result(self) -> anyhow::Result<()> {
        self.error.map_or(Ok(()), Err)
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: anyhow::Error) {
        tracing::error!("{error:#}");
        self.error.get_or_insert(error);
        event_loop.exit();
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Specular (Rust spike)"))
                .context("creating window")?,
        );
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(Arc::clone(&window))
            .context("creating surface")?;
        let context = pollster::block_on(GpuContext::new(instance, Some(&surface)))?;
        let size = window.inner_size();
        let config = surface
            .get_default_config(&context.adapter, size.width.max(1), size.height.max(1))
            .context("surface unsupported by adapter")?;
        surface.configure(&context.device, &config);
        let compositor =
            Compositor::new(context.device.clone(), context.queue.clone(), config.format);
        self.gpu = Some(Gpu {
            window,
            surface,
            config,
            context,
            compositor,
        });

        for node in std::mem::take(&mut self.initial_pages) {
            let viewport = CssSize::new(node.rect.width as u32, node.rect.height as u32);
            let page = self
                .source
                .create_page(&PageSpec::new(&node.url, viewport))?;
            self.draws.push(PageDraw {
                page,
                rect: node.rect,
            });
        }
        Ok(())
    }

    fn redraw(&mut self) -> anyhow::Result<()> {
        self.source.pump();
        self.source.drain_events(&mut self.events);
        let Some(gpu) = self.gpu.as_mut() else {
            return Ok(());
        };
        for event in self.events.drain(..) {
            gpu.compositor.handle_page_event(event)?;
        }
        let frame = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                gpu.surface.configure(&gpu.context.device, &gpu.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                gpu.surface.configure(&gpu.context.device, &gpu.config);
                return Ok(());
            }
            _ => return Ok(()),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let scale_factor = gpu.window.scale_factor() as f32;
        let viewport = Vec2::new(gpu.config.width as f32, gpu.config.height as f32) / scale_factor;
        gpu.compositor.render(
            &view,
            &SceneView {
                camera: self.camera,
                viewport,
                scale_factor,
                pages: &self.draws,
                grid: DotGrid::default(),
            },
        );
        gpu.window.pre_present_notify();
        gpu.context.queue.present(frame);
        Ok(())
    }

    fn on_wheel(&mut self, delta: MouseScrollDelta) {
        let delta = match delta {
            MouseScrollDelta::LineDelta(x, y) => Vec2::new(x, y) * PIXELS_PER_LINE,
            MouseScrollDelta::PixelDelta(position) => {
                let scale = self
                    .gpu
                    .as_ref()
                    .map_or(1.0, |gpu| gpu.window.scale_factor());
                let logical = position.to_logical::<f32>(scale);
                Vec2::new(logical.x, logical.y)
            }
        };
        let input = if self.zoom_modifier {
            ViewportInputDelta {
                zoom_delta_y: -delta.y,
                ..ViewportInputDelta::default()
            }
        } else {
            ViewportInputDelta {
                pan: delta,
                ..ViewportInputDelta::default()
            }
        };
        self.camera.apply_input_delta(input);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Err(error) = self.init(event_loop) {
            self.fail(event_loop, error);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.source.shutdown();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.config.width = size.width.max(1);
                    gpu.config.height = size.height.max(1);
                    gpu.surface.configure(&gpu.context.device, &gpu.config);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.zoom_modifier = state.super_key() || state.control_key();
            }
            WindowEvent::MouseWheel { delta, .. } => self.on_wheel(delta),
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw() {
                    self.fail(event_loop, error);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.request_redraw();
        }
    }
}
