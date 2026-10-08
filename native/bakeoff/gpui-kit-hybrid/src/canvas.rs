//! The canvas: Specular's own compositor drawing a real `.canvas` into a
//! wgpu surface on the child view's CAMetalLayer.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use glam::Vec2;
use specular_compositor::{Compositor, DotGrid, FrameView, GpuContext};
use specular_core::{Camera, InputEvent, PageEvent, PageId, PageSource, PageSpec};
use specular_doc::EntityId;
use specular_interact::{Action, Effect};
use specular_testkit::TestApp;

use crate::log::{self, Pacing};
use crate::native::NativeCanvas;

thread_local! {
    static CANVAS: RefCell<Option<Canvas>> = const { RefCell::new(None) };
}

/// What asks the canvas for a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    /// The canvas view's own display link, independent of GPUI's frames.
    Link,
    /// GPUI's frame: the canvas draws from the element's paint.
    Gpui,
}

pub struct Canvas {
    pub native: NativeCanvas,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    gpu: GpuContext,
    compositor: Compositor,
    app: TestApp,
    hosts: HashMap<EntityId, PageId>,
    pub source: Box<dyn PageSource>,
    pub pacing: Pacing,
    pub drive: Drive,
    /// Pans the camera a little each frame, so a stalled frame shows.
    pub animate: bool,
    started: Instant,
    base_camera: Camera,
    /// The view's size in points.
    size: (f64, f64),
    scale: f64,
    pub page_frames: u64,
    pub gpu_shared_frames: u64,
    pub reconfigures: u32,
    pub reconfigure_time: Duration,
    pub render_time: Duration,
    pub renders: u32,
    /// Whether `frame` turns the page source's message loop itself.
    pub pump_in_frame: bool,
    focused: Option<PageId>,
    /// Frames asked for that got no drawable.
    pub skipped: u32,
    /// Display-link callbacks received.
    pub ticks: u32,
    logged_error: bool,
}

/// Runs `with` on the installed canvas, unless it is absent or busy.
pub fn with<R>(with: impl FnOnce(&mut Canvas) -> R) -> Option<R> {
    CANVAS.with(|cell| {
        let mut slot = cell.try_borrow_mut().ok()?;
        slot.as_mut().map(with)
    })
}

pub fn install(canvas: Canvas) {
    CANVAS.with(|cell| *cell.borrow_mut() = Some(canvas));
}

pub fn uninstall() -> Option<Canvas> {
    CANVAS.with(|cell| cell.borrow_mut().take())
}

/// Called by the display link on the main thread, once per refresh.
pub fn display_link_fired(_target: f64) {
    with(|canvas| {
        canvas.ticks += 1;
        if canvas.drive == Drive::Link {
            canvas.frame();
        }
    });
}

impl Canvas {
    pub fn new(
        native: NativeCanvas,
        source: Box<dyn PageSource>,
        canvas_json: &str,
        drive: Drive,
    ) -> anyhow::Result<Self> {
        let scale = native.scale();
        native.set_contents_scale(scale);
        let size = native.content_size();
        let instance = wgpu::Instance::default();
        // SAFETY: `native.layer` is a live CAMetalLayer that the view
        // retains for as long as this canvas (and so the surface) exists.
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(
                native.layer.cast(),
            ))
        }
        .context("creating a surface on the child view's layer")?;
        let gpu = pollster::block_on(GpuContext::new(instance, Some(&surface)))?;
        let pixels = |points: f64| ((points * scale).round() as u32).max(1);
        let mut config = surface
            .get_default_config(&gpu.adapter, pixels(size.0), pixels(size.1))
            .context("surface unsupported by adapter")?;
        let plain = config.format.remove_srgb_suffix();
        if surface
            .get_capabilities(&gpu.adapter)
            .formats
            .contains(&plain)
        {
            config.format = plain;
        }
        config.present_mode = wgpu::PresentMode::AutoVsync;
        surface.configure(&gpu.device, &config);
        let mut compositor = Compositor::new(gpu.device.clone(), gpu.queue.clone(), config.format);
        compositor.warm_text();

        let document = TestApp::from_canvas(canvas_json).document().clone();
        let mut app = TestApp::empty();
        app.measure_with(Arc::new(compositor.text_measure()));
        app.viewport(Vec2::new(size.0 as f32, size.1 as f32))
            .with_panels()
            .open(document)
            .act(Action::ZoomToFit);
        let base_camera = app.session().camera;
        log::line(format!(
            "canvas adapter={} format={:?} present={:?} scale={scale} size={size:?}",
            gpu.adapter.get_info().name,
            config.format,
            config.present_mode
        ));
        let mut canvas = Self {
            native,
            surface,
            config,
            gpu,
            compositor,
            app,
            hosts: HashMap::new(),
            source,
            pacing: Pacing::default(),
            drive,
            animate: true,
            started: Instant::now(),
            base_camera,
            size,
            scale,
            page_frames: 0,
            gpu_shared_frames: 0,
            reconfigures: 0,
            reconfigure_time: Duration::ZERO,
            render_time: Duration::ZERO,
            renders: 0,
            pump_in_frame: true,
            focused: None,
            skipped: 0,
            ticks: 0,
            logged_error: false,
        };
        canvas.run_effects();
        Ok(canvas)
    }

    /// Hosts the pages the app asked for.
    fn run_effects(&mut self) {
        for effect in self.app.take_effects() {
            match effect {
                Effect::CreatePage {
                    page,
                    url,
                    viewport,
                } => {
                    let mut spec = PageSpec::new(&url, viewport);
                    spec.texture_scale = self.scale as f32;
                    match self.source.create_page(&spec) {
                        Ok(host) => {
                            log::line(format!("canvas hosting {url} as {host:?}"));
                            self.hosts.insert(page, host);
                        }
                        Err(error) => log::line(format!("canvas could not host {url}: {error}")),
                    }
                }
                Effect::ClosePage(page) => {
                    if let Some(host) = self.hosts.remove(&page) {
                        let _ = self.source.close_page(host);
                        self.compositor.remove_page(host);
                    }
                }
                _ => {}
            }
        }
    }

    /// Moves and sizes the view. `x`, `y` are from the content view's top-left.
    pub fn set_rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.native.set_frame_top_left(x, y, width, height);
        if (width, height) == self.size {
            return;
        }
        self.size = (width, height);
        let pixels = |points: f64| ((points * self.scale).round() as u32).max(1);
        self.config.width = pixels(width);
        self.config.height = pixels(height);
        let began = Instant::now();
        self.surface.configure(&self.gpu.device, &self.config);
        self.reconfigure_time += began.elapsed();
        self.reconfigures += 1;
        self.app.viewport(Vec2::new(width as f32, height as f32));
    }

    pub fn app(&mut self) -> &mut TestApp {
        &mut self.app
    }

    /// Pumps the page source and takes its frames.
    fn take_page_events(&mut self) {
        let mut events = Vec::new();
        if self.pump_in_frame {
            self.source.pump();
        }
        self.source.drain_events(&mut events);
        for event in events {
            if let PageEvent::Frame(frame) = &event {
                self.page_frames += 1;
                if matches!(frame.frame, specular_core::PageFrame::GpuShared(_)) {
                    self.gpu_shared_frames += 1;
                }
            }
            if let Err(error) = self.compositor.handle_page_event(event)
                && !self.logged_error
            {
                self.logged_error = true;
                log::line(format!("canvas page event failed: {error}"));
            }
        }
    }

    /// Renders and presents one frame.
    pub fn frame(&mut self) {
        let began = Instant::now();
        self.take_page_events();
        self.run_effects();
        if self.animate {
            let t = self.started.elapsed().as_secs_f32();
            let mut camera = self.base_camera;
            camera.pan_by(Vec2::new(120.0 * (t * 1.5).sin(), 40.0 * (t * 0.9).cos()));
            self.app.act(Action::SetCamera(camera));
        }
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            other => {
                if self.skipped == 0 {
                    log::line(format!(
                        "canvas no drawable ({:?}) after {} frames",
                        std::mem::discriminant(&other),
                        self.renders
                    ));
                }
                self.skipped += 1;
                self.surface.configure(&self.gpu.device, &self.config);
                return;
            }
        };
        let viewport = Vec2::new(self.size.0 as f32, self.size.1 as f32);
        let mut scene = specular_scene::view(self.app.app(), viewport);
        specular_scene::draw_panels(self.app.app(), &mut scene);
        let frame_view = FrameView {
            camera: self.app.session().camera,
            viewport,
            scale_factor: self.scale as f32,
            grid: DotGrid::default(),
            zooming: false,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let hosts = &self.hosts;
        self.compositor
            .render_scene(&view, &frame_view, &scene, |entity| {
                hosts.get(entity).copied()
            });
        self.gpu.queue.present(frame);
        self.render_time += began.elapsed();
        self.renders += 1;
        self.pacing.mark();
    }

    /// Gives keyboard focus to the page behind `entity`, as entering it does.
    pub fn focus_page(&mut self, entity: &str) -> bool {
        let Some((_, &host)) = self.hosts.iter().find(|(id, _)| id.as_str() == entity) else {
            return false;
        };
        self.focused = Some(host);
        self.source.set_focus(Some(host)).is_ok()
    }

    /// Sends input to the focused page.
    pub fn send_to_page(&mut self, event: &InputEvent) {
        let Some(host) = self.focused else {
            log::line("canvas no page has focus".to_owned());
            return;
        };
        match self.source.send_input(host, event) {
            Ok(()) => log::line(format!("page   <- {event:?}")),
            Err(error) => log::line(format!("canvas page input failed: {error}")),
        }
    }

    pub fn close(mut self) {
        self.source.shutdown();
        self.native.close();
    }
}
