//! The winit application: one window, one compositor, one page source.
//!
//! Each loop turn: advance the bench (if any), pump the source, feed its
//! events to the compositor, render, present, then report a
//! [`FrameSample`].

mod api_run;
mod asset_run;
mod bench;
mod clipboard_run;
mod drop_run;
mod effects;
#[cfg(target_os = "macos")]
mod file_menu;
mod gpu_window;
mod image_run;
mod input;
mod lod;
#[cfg(target_os = "macos")]
mod menu_bar;
mod note_run;
mod page_events;
mod settings;
mod title;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use glam::Vec2;
use specular_bench::{GestureProfile, PaintPolicy, STEP_INTERVAL};
use specular_compositor::{FrameObserver as _, FrameSample};
use specular_core::{Camera, PageEvent, PageId, PageSource};
use specular_doc::{Document, EntityId};
use specular_interact::{Action, ApiOutcome, App, Event, ImageKey};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

pub(crate) use self::api_run::ShellEvent;
use self::gpu_window::GpuWindow;
use crate::api::ApiHost;
use crate::bench_run::BenchRun;
use crate::images::ImageLoader;
use crate::latency::InputLatencyProbe;
use crate::notes::NoteLoader;
use crate::page_queries::PageQueries;
use crate::paint_lod::PageLod;
use crate::persist::{self, Persistence};
use crate::prefs;
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
    /// The folder the document's relative file paths start from, and where
    /// pasted and dropped files go: the folder its `.canvas` file is in.
    space: Option<PathBuf>,
    /// The file the document is saved to and reloaded from. `None` for a
    /// demo grid, and for a run that must not write: a benchmark, or one
    /// with seeded annotations in the document.
    persist: Option<Persistence>,
    app: App,
    /// The hosted page behind each page entity.
    hosts: HashMap<EntityId, PageHost>,
    /// What pages have been asked and not yet answered.
    queries: PageQueries,
    /// The decode thread. `None` if it could not be started; every image
    /// then stays a placeholder.
    image_loader: Option<ImageLoader>,
    /// The images the app has asked for and not let go of.
    images: HashSet<ImageKey>,
    /// The thread that reads and watches markdown files. `None` if it could
    /// not be started; every Document then stays on its loading line.
    note_loader: Option<NoteLoader>,
    /// The Document heights the app was last told, to tell it only changes.
    note_heights: HashMap<EntityId, f32>,
    /// The system clipboard, once something has been copied or pasted.
    clipboard: Option<arboard::Clipboard>,
    /// The preferences file. `None` in a benchmark, which neither reads nor
    /// writes settings, and when there is no home folder to keep it in.
    prefs: Option<PathBuf>,
    /// Files dropped on the window this turn, not yet sent to the app.
    dropped: Vec<PathBuf>,
    /// Wakes the event loop from another thread.
    wake: EventLoopProxy<ShellEvent>,
    /// The HTTP API. `None` in a benchmark, and when it could not start.
    api: Option<ApiHost>,
    /// How the API call being run went, from its reply effect.
    api_outcome: Option<ApiOutcome>,
    #[cfg(target_os = "macos")]
    menu: Option<menu_bar::MenuBar>,
    /// The window title as last set.
    title: String,
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
        wake: EventLoopProxy<ShellEvent>,
    ) -> Self {
        // A benchmark starts every run from the same camera and leaves the
        // file as it found it.
        let editing = options.bench.is_none() && options.annotations == 0;
        let canvas = options.canvas.as_deref().filter(|_| editing);
        let start_camera = canvas
            .and_then(|_| persist::camera_of(&document))
            .unwrap_or(START_CAMERA);
        let prefs = (options.bench.is_none()).then(prefs::file).flatten();
        Self {
            source,
            document: Some(document),
            start_camera,
            space: options.canvas.as_deref().and_then(image_run::space_folder),
            persist: canvas.map(Persistence::open),
            app: App::new(unix_ms()),
            hosts: HashMap::new(),
            queries: PageQueries::default(),
            image_loader: image_run::start_loader(options.canvas.as_deref()),
            images: HashSet::new(),
            note_loader: note_run::start_loader(options.canvas.as_deref()),
            note_heights: HashMap::new(),
            clipboard: None,
            prefs,
            dropped: Vec::new(),
            wake,
            api: None,
            api_outcome: None,
            #[cfg(target_os = "macos")]
            menu: None,
            title: String::new(),
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

    /// Reports the session, writes what is unsaved, then starts shutting
    /// down. The event loop keeps turning until the source reports it is
    /// done (see `about_to_wait`).
    fn exit(&mut self) {
        if self.closing {
            return;
        }
        self.report_session();
        // The discovery file goes with the server.
        self.api = None;
        self.finish_notes();
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
        // Before the document opens, so its text is sized with these fonts.
        self.app
            .set_text_measure(std::sync::Arc::new(gpu.compositor.text_measure()));
        self.gpu = Some(gpu);
        self.dispatch(Event::ViewportResized(viewport));
        self.dispatch(Event::Action(Action::SetCamera(self.start_camera)));
        self.load_tool_defaults();
        if let Some(document) = self.document.take() {
            self.dispatch(Event::DocumentOpened(Box::new(document)));
        }
        let Some(profiles) = self.options.bench.take() else {
            // A benchmark keeps winit's default menu: fewer moving parts in
            // a measured run.
            #[cfg(target_os = "macos")]
            self.install_menu();
            let wake = self.wake.clone();
            self.start_api(&wake);
            return Ok(());
        };
        if !self.closing {
            self.start_bench(profiles, step_interval);
        }
        Ok(())
    }

    fn redraw(&mut self) -> anyhow::Result<()> {
        let Some(viewport) = self.gpu.as_ref().map(GpuWindow::logical_viewport) else {
            return Ok(());
        };
        if self.tick_bench(viewport)? {
            return Ok(());
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
        self.report_note_heights();
        Ok(())
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
}

/// Milliseconds since the Unix epoch, for the app's clock.
fn unix_ms() -> u64 {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    since_epoch.as_millis() as u64
}

impl ApplicationHandler<ShellEvent> for Shell {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: ShellEvent) {
        match event {
            ShellEvent::Api => self.serve_api(),
        }
    }

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
            WindowEvent::DroppedFile(path) => self.dropped.push(path),
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
        self.flush_drops();
        #[cfg(target_os = "macos")]
        self.run_menu();
        self.refresh_title();
        if let Some(gpu) = self.gpu.as_ref() {
            gpu.window.request_redraw();
        }
    }
}
