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
use specular_bench::{BenchLine, GestureProfile, InputLatencyLine, PaintPolicy, STEP_INTERVAL};
use specular_compositor::{FrameObserver as _, FrameSample, PageDraw, ShapeDraw};
use specular_core::document::PageNode;
use specular_core::{Camera, CssSize, InputEvent, PageEvent, PageId, PageSource, PageSpec};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use self::gpu_window::GpuWindow;
use crate::annotation;
use crate::bench_run::{BenchRun, BenchTick, RunSource};
use crate::chrome::{self, ChromeScene};
use crate::chrome_state::ChromeState;
use crate::input_map::{ButtonCapture, ClickCounter};
use crate::latency::InputLatencyProbe;
use crate::paint_lod::LodChange;
use crate::placement::PlacedPage;

/// Camera the canvas opens at (and each bench profile starts from).
const START_CAMERA: Camera = Camera {
    pan: Vec2::new(40.0, 40.0),
    zoom: 0.25,
};

/// How the session runs, from the command line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Session {
    /// Profiles to run then exit; `None` stays interactive.
    pub(crate) bench: Option<Vec<GestureProfile>>,
    /// Settle time before the first bench profile.
    pub(crate) warmup: Duration,
    /// Whether the source's frames may be compared with Electron's.
    pub(crate) representative_source: bool,
    /// How pages are throttled.
    pub(crate) paint_policy: PaintPolicy,
    /// Window size in logical pixels; `None` takes the platform default.
    pub(crate) window: Option<(u32, u32)>,
    /// Whether the chrome layer (borders, selection, annotations, tools) runs.
    pub(crate) chrome: bool,
    /// Page-bound annotations seeded at startup.
    pub(crate) annotations: usize,
}

/// Application state driven by winit.
pub(crate) struct App {
    source: Box<dyn PageSource>,
    initial_pages: Vec<PageNode>,
    placed: Vec<PlacedPage>,
    /// Per-frame scratch rebuilt from `placed`.
    draws: Vec<PageDraw>,
    /// Per-frame chrome shapes, reused across frames.
    shapes: Vec<ShapeDraw>,
    /// `None` with `--chrome off`: no shapes, no tool, no selection.
    chrome: Option<ChromeState>,
    camera: Camera,
    gpu: Option<GpuWindow>,
    events: Vec<PageEvent>,
    key_scratch: Vec<InputEvent>,
    modifiers: ModifiersState,
    /// Cursor position in logical window pixels.
    cursor: Option<Vec2>,
    hovered: Option<PageId>,
    focused: Option<PageId>,
    captured: ButtonCapture,
    clicks: ClickCounter,
    latency: InputLatencyProbe,
    session: Session,
    bench: Option<BenchRun>,
    error: Option<anyhow::Error>,
    /// Set once exit starts; the loop ends when the source has shut down.
    closing: bool,
}

impl App {
    /// An app hosting `initial_pages` in `source`; with bench profiles in
    /// `session` it runs them after the warmup and exits instead of staying
    /// interactive.
    pub(crate) fn new(
        source: Box<dyn PageSource>,
        initial_pages: Vec<PageNode>,
        session: Session,
    ) -> Self {
        Self {
            source,
            initial_pages,
            placed: Vec::new(),
            draws: Vec::new(),
            shapes: Vec::new(),
            chrome: None,
            camera: START_CAMERA,
            gpu: None,
            events: Vec::new(),
            key_scratch: Vec::new(),
            modifiers: ModifiersState::empty(),
            cursor: None,
            hovered: None,
            focused: None,
            captured: ButtonCapture::default(),
            clicks: ClickCounter::default(),
            latency: InputLatencyProbe::default(),
            session,
            bench: None,
            error: None,
            closing: false,
        }
    }

    /// The first fatal error hit inside the event loop, if any.
    pub(crate) fn into_result(self) -> anyhow::Result<()> {
        self.error.map_or(Ok(()), Err)
    }

    fn fail(&mut self, error: anyhow::Error) {
        tracing::error!("{error:#}");
        self.error.get_or_insert(error);
        self.exit();
    }

    /// Reports the session's input latency (a JSON line for `assemble`) and
    /// import-cache use, then starts shutting down. The event loop keeps
    /// turning until the source reports it is done (see `about_to_wait`).
    fn exit(&mut self) {
        if self.closing {
            return;
        }
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
            let line = BenchLine::InputLatency(InputLatencyLine {
                input_latency: latency,
            });
            match serde_json::to_string(&line) {
                Ok(json) => println!("{json}"),
                Err(error) => tracing::warn!("cannot report input latency: {error}"),
            }
        }
        if let Some(gpu) = self.gpu.as_ref() {
            let (hits, misses) = gpu.compositor.import_cache_hits_and_misses();
            if hits + misses > 0 {
                tracing::info!(hits, misses, "shared-surface import cache");
            }
        }
        self.placed.clear();
        self.closing = true;
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let gpu = GpuWindow::new(event_loop, self.session.window)?;
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
            self.placed.push(PlacedPage::new(page, node.rect, viewport));
        }
        if self.session.chrome {
            let mut chrome =
                ChromeState::new(annotation::seed(self.session.annotations, &self.placed));
            if self.session.bench.is_some() {
                // The selection outline and handles belong in every measured frame.
                chrome.select(self.placed.first().map(|placed| placed.page));
            }
            self.chrome = Some(chrome);
        }
        if let Some(profiles) = self.session.bench.take() {
            let step_interval = gpu.refresh_interval().unwrap_or(STEP_INTERVAL);
            self.bench = Some(BenchRun::new(
                profiles,
                self.session.warmup,
                step_interval,
                START_CAMERA,
                RunSource {
                    name: self.source.name(),
                    representative: self.session.representative_source,
                    pages: self.placed.len(),
                    paint_policy: self.session.paint_policy,
                    chrome: self.session.chrome,
                    annotations: self.session.annotations,
                },
                Instant::now(),
            ));
        }
        self.gpu = Some(gpu);
        Ok(())
    }

    fn redraw(&mut self) -> anyhow::Result<()> {
        let Some(viewport) = self.gpu.as_ref().map(GpuWindow::logical_viewport) else {
            return Ok(());
        };
        if let Some(bench) = self.bench.as_mut()
            && bench.tick(Instant::now(), &mut self.camera, viewport / 2.0) == BenchTick::Finished
        {
            self.finish_bench()?;
            return Ok(());
        }

        if self.session.paint_policy == PaintPolicy::ElectronLod {
            self.update_paint_lod(viewport, Instant::now());
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
        self.draws.clear();
        self.draws.extend(self.placed.iter().map(PlacedPage::draw));
        let scene = self.chrome.as_ref().map(|chrome| ChromeScene {
            placed: &self.placed,
            selected: chrome.selected(),
            hovered: self.hovered,
            annotations: chrome.annotations(),
            preview: chrome.preview(),
        });
        chrome::build_shapes(scene.as_ref(), &mut self.shapes);
        let Some(stats) = gpu.render(self.camera, &self.draws, &self.shapes) else {
            return Ok(());
        };
        let presented_at = Instant::now();
        let sample = FrameSample {
            presented_at,
            stats,
            input_to_present: self.latency.presented(presented_at),
        };
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
            | PageEvent::FrameDropped { .. }
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

    fn finish_bench(&mut self) -> anyhow::Result<()> {
        if let Some(bench) = self.bench.take() {
            for report in bench.reports() {
                println!("{}", serde_json::to_string(report)?);
            }
        }
        self.exit();
        Ok(())
    }

    fn on_scale_factor_changed(&mut self, scale_factor: f64) {
        for placed in &self.placed {
            let scale = scale_factor as f32 * placed.lod.texture().factor();
            if let Err(error) = self.source.set_texture_scale(placed.page, scale) {
                tracing::warn!("{error}");
            }
        }
    }

    /// One layout pass of the Electron page-host LOD: grades every page by
    /// its on-screen scale and visibility and applies what changed.
    fn update_paint_lod(&mut self, viewport: Vec2, now: Instant) {
        let window_scale = self.gpu.as_ref().map_or(1.0, GpuWindow::scale_factor);
        for placed in &mut self.placed {
            let on_screen = self.camera.is_visible(placed.rect, viewport);
            let display_scale = placed.display_scale(&self.camera);
            let change = placed.lod.update(display_scale, on_screen, now);
            apply_lod_change(self.source.as_mut(), placed.page, change, window_scale);
        }
    }
}

/// Applies `change` in the order Electron's layout pass does: scale before
/// painting, so a page coming into view wakes at the scale it is owed.
fn apply_lod_change(
    source: &mut dyn PageSource,
    page: PageId,
    change: LodChange,
    window_scale: f32,
) {
    if change != LodChange::default() {
        tracing::debug!(%page, ?change, "paint LOD");
    }
    let results = [
        change
            .texture
            .map(|tier| source.set_texture_scale(page, window_scale * tier.factor())),
        change
            .frame_rate
            .map(|fps| source.set_frame_rate(page, fps)),
        change
            .painting
            .map(|painting| source.set_painting(page, painting)),
    ];
    for error in results.into_iter().flatten().filter_map(Result::err) {
        tracing::warn!("{error}");
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_some() {
            return;
        }
        event_loop.set_control_flow(ControlFlow::Poll);
        if let Err(error) = self.init(event_loop) {
            self.fail(error);
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.resize(size);
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.on_scale_factor_changed(scale_factor);
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw() {
                    self.fail(error);
                }
            }
            other => self.on_input(other),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.closing {
            if self.source.poll_shutdown() {
                event_loop.exit();
            }
            return;
        }
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.request_redraw();
        }
    }
}
