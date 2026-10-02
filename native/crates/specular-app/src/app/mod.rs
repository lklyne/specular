//! The winit application: one window, one compositor, one page source.
//!
//! Each loop turn: advance the bench (if any), pump the source, feed its
//! events to the compositor, render, present, then report a
//! [`FrameSample`].

mod gpu_window;
mod input;

use std::time::{Duration, Instant};

use anyhow::Context as _;
use glam::Vec2;
use specular_bench::{GestureProfile, STEP_INTERVAL};
use specular_compositor::{FrameObserver as _, FrameSample, PageDraw};
use specular_core::document::PageNode;
use specular_core::{Camera, CssSize, InputEvent, PageEvent, PageId, PageSource, PageSpec};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use self::gpu_window::GpuWindow;
use crate::bench_run::{BenchRun, BenchTick};
use crate::input_map::ClickCounter;
use crate::latency::InputLatencyProbe;
use crate::placement::PlacedPage;

/// Camera the canvas opens at (and each bench profile starts from).
const START_CAMERA: Camera = Camera {
    pan: Vec2::new(40.0, 40.0),
    zoom: 0.25,
};

/// Application state driven by winit.
pub(crate) struct App {
    source: Box<dyn PageSource>,
    initial_pages: Vec<PageNode>,
    placed: Vec<PlacedPage>,
    draws: Vec<PageDraw>,
    camera: Camera,
    gpu: Option<GpuWindow>,
    events: Vec<PageEvent>,
    key_scratch: Vec<InputEvent>,
    modifiers: ModifiersState,
    /// Cursor position in logical window pixels.
    cursor: Option<Vec2>,
    hovered: Option<PageId>,
    focused: Option<PageId>,
    clicks: ClickCounter,
    latency: InputLatencyProbe,
    bench_profiles: Option<Vec<GestureProfile>>,
    bench_warmup: Duration,
    bench: Option<BenchRun>,
    last_present: Option<Instant>,
    error: Option<anyhow::Error>,
}

impl App {
    /// An app hosting `initial_pages` in `source`; with `bench_profiles` it
    /// runs them after `bench_warmup` and exits instead of staying
    /// interactive.
    pub(crate) fn new(
        source: Box<dyn PageSource>,
        initial_pages: Vec<PageNode>,
        bench_profiles: Option<Vec<GestureProfile>>,
        bench_warmup: Duration,
    ) -> Self {
        Self {
            source,
            initial_pages,
            placed: Vec::new(),
            draws: Vec::new(),
            camera: START_CAMERA,
            gpu: None,
            events: Vec::new(),
            key_scratch: Vec::new(),
            modifiers: ModifiersState::empty(),
            cursor: None,
            hovered: None,
            focused: None,
            clicks: ClickCounter::default(),
            latency: InputLatencyProbe::default(),
            bench_profiles,
            bench_warmup,
            bench: None,
            last_present: None,
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
        self.exit(event_loop);
    }

    /// Reports the session's input latency, then shuts the source down.
    fn exit(&mut self, event_loop: &ActiveEventLoop) {
        let latency = self.latency.summary();
        if latency.samples > 0 {
            tracing::info!(
                samples = latency.samples,
                unresolved = latency.unresolved,
                p50_ms = latency.p50_ms,
                p95_ms = latency.p95_ms,
                max_ms = latency.max_ms,
                "input to present"
            );
        }
        self.source.shutdown();
        event_loop.exit();
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let gpu = GpuWindow::new(event_loop)?;
        let texture_scale = gpu.scale_factor();
        for node in std::mem::take(&mut self.initial_pages) {
            let viewport = CssSize::new(
                node.rect.width.round().max(1.0) as u32,
                node.rect.height.round().max(1.0) as u32,
            );
            let mut spec = PageSpec::new(&node.url, viewport);
            spec.texture_scale = texture_scale;
            let page = self
                .source
                .create_page(&spec)
                .with_context(|| format!("creating page for {}", node.url))?;
            self.placed.push(PlacedPage {
                page,
                rect: node.rect,
                viewport,
            });
            self.draws.push(PageDraw {
                page,
                rect: node.rect,
            });
        }
        if let Some(profiles) = self.bench_profiles.take() {
            let step_interval = gpu.refresh_interval().unwrap_or(STEP_INTERVAL);
            self.bench = Some(BenchRun::new(
                profiles,
                self.bench_warmup,
                step_interval,
                START_CAMERA,
                self.source.name(),
                self.placed.len(),
                Instant::now(),
            ));
        }
        self.gpu = Some(gpu);
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let Some(viewport) = self.gpu.as_ref().map(GpuWindow::logical_viewport) else {
            return Ok(());
        };
        if let Some(bench) = self.bench.as_mut()
            && bench.tick(Instant::now(), &mut self.camera, viewport / 2.0) == BenchTick::Finished
        {
            self.finish_bench(event_loop)?;
            return Ok(());
        }

        self.source.pump();
        let mut events = std::mem::take(&mut self.events);
        self.source.drain_events(&mut events);
        for event in events.drain(..) {
            self.handle_page_event(event);
        }
        self.events = events;

        let Some(gpu) = self.gpu.as_mut() else {
            return Ok(());
        };
        let Some(stats) = gpu.render(self.camera, &self.draws) else {
            return Ok(());
        };
        let presented_at = Instant::now();
        let sample = FrameSample {
            presented_at,
            interval: self
                .last_present
                .map(|last| presented_at.saturating_duration_since(last)),
            stats,
            input_to_present: self.latency.presented(presented_at),
        };
        self.last_present = Some(presented_at);
        if let Some(latency) = sample.input_to_present {
            tracing::debug!(?latency, "input to present");
        }
        if let Some(bench) = self.bench.as_mut() {
            bench.on_frame(&sample);
        }
        Ok(())
    }

    fn handle_page_event(&mut self, event: PageEvent) {
        self.latency.observe(&event);
        match &event {
            PageEvent::Loaded { page, http_status } => {
                tracing::info!(%page, http_status, "page loaded");
            }
            PageEvent::Crashed { page, reason } => {
                tracing::error!(%page, reason, "page host crashed");
            }
            PageEvent::ImeCompositionBounds { page, bounds } => {
                if self.focused == Some(*page)
                    && let Some(bounds) = bounds
                {
                    self.place_ime_candidates(*page, *bounds);
                }
            }
            PageEvent::Frame(_)
            | PageEvent::PopupVisibility { .. }
            | PageEvent::PopupRect { .. } => {}
        }
        if let Some(gpu) = self.gpu.as_mut()
            && let Err(error) = gpu.compositor.handle_page_event(event)
        {
            tracing::warn!("{error}");
        }
    }

    /// Moves the OS candidate window next to the page's composition.
    fn place_ime_candidates(&self, page: PageId, bounds: specular_core::PixelRect) {
        let (Some(gpu), Some(placed)) = (
            self.gpu.as_ref(),
            self.placed.iter().find(|placed| placed.page == page),
        ) else {
            return;
        };
        let scale = placed.canvas_per_css() * self.camera.zoom;
        let world = placed.rect.origin()
            + Vec2::new(bounds.x as f32, bounds.y as f32) * placed.canvas_per_css();
        let screen = self.camera.world_to_screen(world);
        let size = Vec2::new(bounds.width as f32, bounds.height as f32) * scale;
        gpu.window.set_ime_cursor_area(
            LogicalPosition::new(screen.x, screen.y),
            LogicalSize::new(size.x, size.y),
        );
    }

    fn finish_bench(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        if let Some(bench) = self.bench.take() {
            for report in bench.reports() {
                println!("{}", serde_json::to_string(report)?);
            }
        }
        self.exit(event_loop);
        Ok(())
    }

    fn on_scale_factor_changed(&mut self, scale_factor: f64) {
        for placed in &self.placed {
            if let Err(error) = self
                .source
                .set_texture_scale(placed.page, scale_factor as f32)
            {
                tracing::warn!("{error}");
            }
        }
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
            WindowEvent::CloseRequested => self.exit(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.resize(size);
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.on_scale_factor_changed(scale_factor);
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw(event_loop) {
                    self.fail(event_loop, error);
                }
            }
            other => self.on_input(other),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.request_redraw();
        }
    }
}
