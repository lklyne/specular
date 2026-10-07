//! The winit application: one window, one compositor, one page source.
//!
//! Each loop turn: advance the bench (if any), pump the source, feed its
//! events to the compositor, render, present, then report a
//! [`FrameSample`].

mod effects;
mod gpu_window;
mod image_run;
mod input;
mod note_run;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use glam::Vec2;
use specular_bench::{BenchLine, GestureProfile, InputLatencyLine, PaintPolicy, STEP_INTERVAL};
use specular_compositor::{FrameObserver as _, FrameSample};
use specular_core::{Camera, PageEvent, PageId, PageSource};
use specular_doc::{Document, EntityId, ItemId};
use specular_interact::{Action, App, Event, ImageKey, PageNotice, to_canvas_rect};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use self::gpu_window::GpuWindow;
use crate::bench_run::{BenchRun, BenchTick, RunSource};
use crate::images::ImageLoader;
use crate::latency::InputLatencyProbe;
use crate::notes::NoteLoader;
use crate::paint_lod::{LodChange, PageLod};
use crate::persist::{self, Persistence};
use crate::translate::ClickCounter;

/// Camera a canvas with no saved one opens at, and each bench profile starts
/// from.
const START_CAMERA: Camera = Camera {
    pan: Vec2::new(40.0, 40.0),
    zoom: 0.25,
};

/// How the shell runs, from the command line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunOptions {
    /// The `.canvas` file the document came from, if it came from one.
    pub(crate) canvas: Option<PathBuf>,
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

/// The backend's side of one page entity.
#[derive(Debug, Clone, Copy)]
struct PageHost {
    page: PageId,
    lod: PageLod,
}

/// The winit shell: turns window and page events into [`Event`]s for the
/// [`App`], runs the effects that come back, and draws.
pub(crate) struct Shell {
    source: Box<dyn PageSource>,
    /// The document to open once the window exists.
    document: Option<Document>,
    /// The camera the canvas opens at.
    start_camera: Camera,
    /// The file the document is saved to and reloaded from. `None` for a
    /// demo grid, and for a run that must not write: a benchmark, or one
    /// with seeded annotations in the document.
    persist: Option<Persistence>,
    app: App,
    /// The hosted page behind each page entity.
    hosts: HashMap<EntityId, PageHost>,
    /// The decode thread. `None` if it could not be started; every image
    /// then stays a placeholder.
    image_loader: Option<ImageLoader>,
    /// The images the app has asked for and not let go of.
    images: HashSet<ImageKey>,
    /// The thread that reads and watches markdown files. `None` if it could
    /// not be started; every Document then stays on its loading line.
    note_loader: Option<NoteLoader>,
    /// The zoom the previous frame was drawn at, to tell when a zoom is in
    /// flight.
    drawn_zoom: f32,
    gpu: Option<GpuWindow>,
    events: Vec<PageEvent>,
    modifiers: ModifiersState,
    /// The pointer's latest position in logical window pixels.
    cursor: Option<Vec2>,
    clicks: ClickCounter,
    latency: InputLatencyProbe,
    options: RunOptions,
    bench: Option<BenchRun>,
    error: Option<anyhow::Error>,
    /// Set once exit starts; the loop ends when the source has shut down.
    closing: bool,
}

impl Shell {
    /// A shell showing `document` in `source`; with bench profiles in
    /// `options` it runs them after the warmup and exits instead of staying
    /// interactive.
    pub(crate) fn new(
        source: Box<dyn PageSource>,
        document: Document,
        options: RunOptions,
    ) -> Self {
        // A benchmark starts every run from the same camera and leaves the
        // file as it found it.
        let editing = options.bench.is_none() && options.annotations == 0;
        let canvas = options.canvas.as_deref().filter(|_| editing);
        let start_camera = canvas
            .and_then(|_| persist::camera_of(&document))
            .unwrap_or(START_CAMERA);
        Self {
            source,
            document: Some(document),
            start_camera,
            persist: canvas.map(Persistence::open),
            app: App::new(unix_ms()),
            hosts: HashMap::new(),
            image_loader: image_run::start_loader(options.canvas.as_deref()),
            images: HashSet::new(),
            note_loader: note_run::start_loader(options.canvas.as_deref()),
            drawn_zoom: start_camera.zoom,
            gpu: None,
            events: Vec::new(),
            modifiers: ModifiersState::empty(),
            cursor: None,
            clicks: ClickCounter::default(),
            latency: InputLatencyProbe::default(),
            options,
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
        if let Some(persist) = self.persist.as_mut() {
            persist.flush(&self.app);
        }
        self.hosts.clear();
        self.closing = true;
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let gpu = GpuWindow::new(
            event_loop,
            self.options.window,
            self.options.bench.is_some(),
        )?;
        let viewport = gpu.logical_viewport();
        let step_interval = gpu.refresh_interval().unwrap_or(STEP_INTERVAL);
        self.gpu = Some(gpu);
        self.dispatch(Event::ViewportResized(viewport));
        self.dispatch(Event::Action(Action::SetCamera(self.start_camera)));
        if let Some(document) = self.document.take() {
            self.dispatch(Event::DocumentOpened(Box::new(document)));
        }
        let Some(profiles) = self.options.bench.take() else {
            return Ok(());
        };
        if self.closing {
            return Ok(());
        }
        if self.options.chrome {
            // The selection outline and handles belong in every measured frame.
            let first = self.app.pages().next().map(|(id, ..)| id.clone());
            let selection = first.into_iter().map(ItemId::Entity).collect();
            self.dispatch(Event::Action(Action::Select(selection)));
        }
        self.bench = Some(BenchRun::new(
            profiles,
            self.options.warmup,
            step_interval,
            START_CAMERA,
            RunSource {
                name: self.source.name(),
                representative: self.options.representative_source,
                pages: self.hosts.len(),
                paint_policy: self.options.paint_policy,
                chrome: self.options.chrome,
                annotations: self.options.annotations,
            },
            Instant::now(),
        ));
        Ok(())
    }

    fn redraw(&mut self) -> anyhow::Result<()> {
        let Some(viewport) = self.gpu.as_ref().map(GpuWindow::logical_viewport) else {
            return Ok(());
        };
        if let Some(bench) = self.bench.as_mut() {
            let mut camera = self.app.session().camera;
            if bench.tick(Instant::now(), &mut camera, viewport / 2.0) == BenchTick::Finished {
                self.finish_bench()?;
                return Ok(());
            }
            if camera != self.app.session().camera {
                self.dispatch(Event::Action(Action::SetCamera(camera)));
            }
        }

        if self.options.paint_policy == PaintPolicy::ElectronLod {
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
        let scene = if self.options.chrome {
            specular_scene::view(&self.app, viewport)
        } else {
            specular_scene::view_without_chrome(&self.app, viewport)
        };
        let camera = self.app.session().camera;
        // Text keeps its raster size while the zoom moves, and the first
        // frame at a steady zoom sharpens it.
        let zooming = (camera.zoom - self.drawn_zoom).abs() > f32::EPSILON;
        self.drawn_zoom = camera.zoom;
        let hosts = &self.hosts;
        let page_of = |entity: &EntityId| hosts.get(entity).map(|host| host.page);
        let Some(stats) = gpu.render(camera, zooming, &scene, page_of) else {
            return Ok(());
        };
        let presented_at = Instant::now();
        let sample = FrameSample {
            presented_at,
            stats: stats.render,
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

    /// Logs what a page reported, tells the app what it acts on, and hands
    /// the event to the compositor for its frames.
    fn handle_page_event(&mut self, event: PageEvent) {
        self.latency.observe(&event);
        let notice = match &event {
            PageEvent::Loaded { page, http_status } => {
                tracing::info!(%page, http_status, "page loaded");
                Some((
                    *page,
                    PageNotice::Loaded {
                        http_status: *http_status,
                    },
                ))
            }
            PageEvent::Crashed { page, reason } => {
                tracing::error!(%page, reason, "page host crashed");
                Some((
                    *page,
                    PageNotice::Crashed {
                        reason: reason.clone(),
                    },
                ))
            }
            PageEvent::ImeCompositionBounds { page, bounds } => {
                Some((*page, PageNotice::ImeCompositionBounds(*bounds)))
            }
            PageEvent::Frame(_)
            | PageEvent::FrameDropped { .. }
            | PageEvent::PopupVisibility { .. }
            | PageEvent::PopupRect { .. } => None,
        };
        if let Some((host, notice)) = notice
            && let Some(page) = self.entity_of(host)
        {
            self.dispatch(Event::Page { page, notice });
        }
        if let Some(gpu) = self.gpu.as_mut()
            && let Err(error) = gpu.compositor.handle_page_event(event)
        {
            tracing::warn!("{error}");
        }
    }

    /// Saves a due autosave, or opens the file again when another tool
    /// edited it. The camera stays, and `update` drops whatever the selection
    /// named that the new document lacks.
    fn sync_file(&mut self) {
        let reloaded = self
            .persist
            .as_mut()
            .and_then(|persist| persist.turn(&self.app));
        if let Some(document) = reloaded {
            self.dispatch(Event::DocumentOpened(Box::new(document)));
        }
    }

    /// The page entity a backend page is hosting.
    fn entity_of(&self, page: PageId) -> Option<EntityId> {
        let (entity, _) = self.hosts.iter().find(|(_, host)| host.page == page)?;
        Some(entity.clone())
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
        for host in self.hosts.values() {
            let scale = scale_factor as f32 * host.lod.texture().factor();
            if let Err(error) = self.source.set_texture_scale(host.page, scale) {
                tracing::warn!("{error}");
            }
        }
    }

    /// One layout pass of the Electron page-host LOD: grades every page by
    /// its on-screen scale and visibility and applies what changed.
    fn update_paint_lod(&mut self, viewport: Vec2, now: Instant) {
        let window_scale = self.gpu.as_ref().map_or(1.0, GpuWindow::scale_factor);
        let camera = self.app.session().camera;
        for (id, _, placement) in self.app.pages() {
            let Some(host) = self.hosts.get_mut(id) else {
                continue;
            };
            let on_screen = camera.is_visible(to_canvas_rect(placement.rect), viewport);
            let display_scale = placement.display_scale(&camera);
            let change = host.lod.update(display_scale, on_screen, now);
            apply_lod_change(self.source.as_mut(), host.page, change, window_scale);
        }
    }
}

/// Milliseconds since the Unix epoch, for the app's clock.
fn unix_ms() -> u64 {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    since_epoch.as_millis() as u64
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

impl ApplicationHandler for Shell {
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
                    let viewport = gpu.logical_viewport();
                    self.dispatch(Event::ViewportResized(viewport));
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
        self.dispatch(Event::Tick { unix_ms: unix_ms() });
        self.sync_file();
        self.take_loaded_image();
        self.take_read_notes();
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.request_redraw();
        }
    }
}
