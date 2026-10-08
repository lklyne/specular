//! The half of a shell that needs no window toolkit: the [`App`], the page
//! backend, the files of the open space, and the runner of every
//! [`Effect`](specular_interact::Effect).
//!
//! A shell owns one [`Runtime`], gives it a [`ShellWindow`] to draw into,
//! and sends it [`Event`]s. `update` is the only thing that changes the
//! app, and [`Runtime::dispatch`] is the only caller of `update`.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use glam::Vec2;
use specular_bench::{FrameWork, PaintPolicy};
use specular_compositor::{Compositor, SceneStats};
use specular_core::{Camera, PageEvent, PageId, PageSource};
use specular_doc::{Document, EntityId};
use specular_interact::{Action, ApiOutcome, App, Cursor, Event, ImageKey};
use specular_scene::Scene;

use super::demand::FrameDemand;
use super::{START_CAMERA, image_run, note_run};
use crate::api::ApiHost;
use crate::cdp::CdpHost;
use crate::images::ImageLoader;
use crate::latency::InputLatencyProbe;
use crate::notes::NoteLoader;
use crate::page_queries::PageQueries;
use crate::paint_lod::PageLod;
use crate::prefs;
use crate::space::{SpaceFiles, SpaceStart};

/// The backend page behind a page entity, for a renderer.
pub type PageOf<'a> = &'a dyn Fn(&EntityId) -> Option<PageId>;

/// What a [`Runtime`] needs of the window it draws into: the compositor,
/// a frame on screen, and the few window calls an effect asks for.
pub trait ShellWindow {
    /// The compositor drawing into this window.
    fn compositor(&self) -> &Compositor;

    /// The compositor, to upload to.
    fn compositor_mut(&mut self) -> &mut Compositor;

    /// Device pixels per logical pixel.
    fn scale_factor(&self) -> f32;

    /// The canvas viewport in logical pixels.
    fn logical_viewport(&self) -> Vec2;

    /// Draws `scene` into a texture of `viewport` logical pixels at the
    /// window's scale, whatever size the window is, and returns the PNG
    /// with its size in device pixels.
    fn capture_area(
        &mut self,
        camera: Camera,
        viewport: Vec2,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)>;

    /// Renders and presents one frame of `scene`; `None` when there was no
    /// frame to draw into. The scene is the window's to rework: one whose
    /// viewport is only part of its surface moves the screen-space items.
    fn render(
        &mut self,
        camera: Camera,
        zooming: bool,
        scene: &mut Scene,
        page_of: PageOf<'_>,
    ) -> Option<SceneStats>;

    /// Draws `scene` into a texture and returns the PNG with its size in
    /// device pixels. The window itself is not touched.
    fn capture(
        &mut self,
        camera: Camera,
        scene: &Scene,
        page_of: PageOf<'_>,
    ) -> anyhow::Result<(Vec<u8>, u32, u32)>;

    /// Whether the input method may compose into the canvas.
    fn set_ime_allowed(&self, allowed: bool);

    /// Where the caret is, in logical pixels of the viewport, for the
    /// input method's candidate window.
    fn set_ime_cursor_area(&self, origin: Vec2, size: Vec2);

    /// The pointer's shape over the canvas.
    fn set_cursor(&self, cursor: Cursor);

    /// The window's title, and whether it shows unsaved changes.
    fn set_title(&self, title: &str, unsaved: bool);
}

/// How a [`Runtime`] runs, from the command line.
#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeOptions {
    /// The `.canvas` file the document came from, when no space is opened.
    /// Its folder is where relative file paths start from.
    pub(crate) canvas: Option<PathBuf>,
    /// How pages are throttled.
    pub(crate) paint_policy: PaintPolicy,
    /// Whether the chrome layer (borders, selection, annotations, tools)
    /// is drawn.
    pub(crate) chrome: bool,
    /// Whether settings are read and written. Not in a benchmark.
    pub(crate) settings: bool,
}

/// What a shell shows first: a space folder, or one document that is saved
/// nowhere.
#[derive(Debug)]
pub struct Opening {
    pub(crate) space: Option<SpaceStart>,
    pub(crate) document: Option<Document>,
}

/// The backend's side of one page entity.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PageHost {
    pub(crate) page: PageId,
    pub(crate) lod: PageLod,
}

/// The app and everything it does I/O through, for a window of type `W`.
pub struct Runtime<W> {
    pub(crate) source: Box<dyn PageSource>,
    /// The folder the document's relative file paths start from, and where
    /// pasted and dropped files go: the folder its `.canvas` file is in.
    pub(crate) space: Option<PathBuf>,
    /// The files of the open space, which its canvases are saved to and
    /// reloaded from. `None` for a demo grid, and for a run that must not
    /// write: a benchmark, or one with seeded annotations in the document.
    pub(crate) files: Option<SpaceFiles>,
    /// The scratch space's folder, when this launch opened it. The title
    /// says so while it is the open space.
    pub(crate) scratch: Option<PathBuf>,
    pub(crate) app: App,
    /// The hosted page behind each page entity.
    pub(crate) hosts: HashMap<EntityId, PageHost>,
    /// What pages have been asked and not yet answered.
    pub(crate) queries: PageQueries,
    /// The decode thread. `None` if it could not be started; every image
    /// then stays a placeholder.
    pub(crate) image_loader: Option<ImageLoader>,
    /// The images the app has asked for and not let go of.
    pub(crate) images: HashSet<ImageKey>,
    /// The thread that reads and watches markdown files. `None` if it could
    /// not be started; every Document then stays on its loading line.
    pub(crate) note_loader: Option<NoteLoader>,
    /// The Document heights the app was last told, to tell it only changes.
    pub(crate) note_heights: HashMap<EntityId, f32>,
    /// What `view` keeps from one frame to the next.
    pub(crate) view_cache: specular_scene::ViewCache,
    /// The system clipboard, once something has been copied or pasted.
    pub(crate) clipboard: Option<arboard::Clipboard>,
    /// The preferences file. `None` when settings are off, and when there
    /// is no home folder to keep it in.
    pub(crate) prefs: Option<PathBuf>,
    /// Files dropped on the window, not yet sent to the app.
    pub(crate) dropped: Vec<PathBuf>,
    /// Where they were dropped, in logical pixels of the viewport.
    pub(crate) dropped_at: Option<Vec2>,
    /// The HTTP API. `None` until started, and when it could not start.
    pub(crate) api: Option<ApiHost>,
    /// Each page's CDP websocket. Started and stopped with the API.
    pub(crate) cdp: Option<CdpHost>,
    /// How the API call being run went, from its reply effect.
    pub(crate) api_outcome: Option<ApiOutcome>,
    /// The window title as last set.
    pub(crate) title: String,
    /// The zoom the previous frame was drawn at, to tell when a zoom is in
    /// flight.
    pub(crate) drawn_zoom: f32,
    /// The window, once it is open.
    pub(crate) gpu: Option<W>,
    pub(crate) events: Vec<PageEvent>,
    pub(crate) latency: InputLatencyProbe,
    /// Whether a frame is owed.
    pub(crate) demand: FrameDemand,
    /// When the per-turn chores last ran.
    pub(crate) chores_at: Option<Instant>,
    /// Where the last drawn frame's time went.
    pub(crate) last_work: FrameWork,
    pub(crate) options: RuntimeOptions,
    pub(crate) error: Option<anyhow::Error>,
    /// Set once exit starts; the shell's loop ends when the source has
    /// shut down.
    pub(crate) closing: bool,
}

impl<W> std::fmt::Debug for Runtime<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime")
            .field("space", &self.space)
            .field("pages", &self.hosts.len())
            .field("closing", &self.closing)
            .finish_non_exhaustive()
    }
}

impl<W: ShellWindow> Runtime<W> {
    /// A runtime hosting pages in `source`, with no window yet and an
    /// empty app.
    pub fn new(source: Box<dyn PageSource>, options: RuntimeOptions) -> Self {
        let space_folder = options.canvas.as_deref().and_then(image_run::space_folder);
        let prefs = options.settings.then(prefs::file).flatten();
        Self {
            source,
            space: space_folder.clone(),
            files: None,
            scratch: None,
            app: App::new(unix_ms()),
            hosts: HashMap::new(),
            queries: PageQueries::default(),
            image_loader: image_run::start_loader(space_folder.clone()),
            images: HashSet::new(),
            note_loader: note_run::start_loader(space_folder),
            note_heights: HashMap::new(),
            view_cache: specular_scene::ViewCache::default(),
            clipboard: None,
            prefs,
            dropped: Vec::new(),
            dropped_at: None,
            api: None,
            cdp: None,
            api_outcome: None,
            title: String::new(),
            drawn_zoom: START_CAMERA.zoom,
            gpu: None,
            events: Vec::new(),
            latency: InputLatencyProbe::default(),
            demand: FrameDemand::default(),
            chores_at: None,
            last_work: FrameWork::default(),
            options,
            error: None,
            closing: false,
        }
    }

    /// The app, to read.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// The window, once [`attach_window`](Self::attach_window) gave one.
    pub fn window(&self) -> Option<&W> {
        self.gpu.as_ref()
    }

    /// The window, to change.
    pub fn window_mut(&mut self) -> Option<&mut W> {
        self.gpu.as_mut()
    }

    /// The folder of the open space, or of the one canvas file shown.
    pub fn space_folder(&self) -> Option<&std::path::Path> {
        self.space.as_deref()
    }

    /// The name of the page backend.
    pub fn source_name(&self) -> &'static str {
        self.source.name()
    }

    /// Whether exit has started.
    pub fn is_closing(&self) -> bool {
        self.closing
    }

    /// Whether the page backend has finished shutting down. Call until it
    /// says so once [`is_closing`](Self::is_closing).
    pub fn poll_shutdown(&mut self) -> bool {
        self.source.poll_shutdown()
    }

    /// The first fatal error hit, if any.
    pub fn into_result(self) -> anyhow::Result<()> {
        self.error.map_or(Ok(()), Err)
    }

    /// Takes the window to draw into. Text is sized with its fonts from
    /// here on, so this comes before anything is opened.
    pub fn attach_window(&mut self, window: W) {
        self.app
            .set_text_measure(std::sync::Arc::new(window.compositor().text_measure()));
        let viewport = window.logical_viewport();
        self.gpu = Some(window);
        self.dispatch(Event::ViewportResized(viewport));
    }

    /// Shows what the launch asked for, at the starting camera.
    pub fn open(&mut self, opening: Opening) -> anyhow::Result<()> {
        self.dispatch(Event::Action(Action::SetCamera(START_CAMERA)));
        self.load_tool_defaults();
        match (opening.space, opening.document) {
            (Some(start), _) => {
                self.open_space(&start.folder, start.file.as_deref())?;
                if start.scratch {
                    tracing::info!(
                        folder = %start.folder.display(),
                        "this is the scratch space, a copy of the starter space; \
                         pass --space user to open your own"
                    );
                    self.scratch = self.space.clone();
                }
            }
            (None, Some(document)) => self.dispatch(Event::DocumentOpened(Box::new(document))),
            (None, None) => {}
        }
        Ok(())
    }

    pub(crate) fn fail(&mut self, error: anyhow::Error) {
        tracing::error!("{error:#}");
        self.error.get_or_insert(error);
        self.exit();
    }

    /// Reports the session, writes what is unsaved, then starts shutting
    /// down. The shell keeps turning until
    /// [`poll_shutdown`](Self::poll_shutdown) says the source is done.
    pub fn exit(&mut self) {
        if self.closing {
            return;
        }
        self.report_session();
        // The discovery file goes with the server.
        self.api = None;
        self.source.set_devtools_sink(None);
        self.cdp = None;
        self.finish_notes();
        self.flush_files();
        self.hosts.clear();
        self.closing = true;
    }

    /// Logs what the session measured: input latency, and how often a
    /// shared surface was imported again.
    fn report_session(&self) {
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
            let line = specular_bench::BenchLine::InputLatency(specular_bench::InputLatencyLine {
                input_latency: latency,
            });
            match serde_json::to_string(&line) {
                Ok(json) => println!("{json}"),
                Err(error) => tracing::warn!("cannot report input latency: {error}"),
            }
        }
        let (drawn, skipped) = self.demand.counts();
        tracing::info!(
            drawn,
            skipped,
            "frames drawn, and turns with nothing to draw"
        );
        if let Some(gpu) = self.gpu.as_ref() {
            let (hits, misses) = gpu.compositor().import_cache_hits_and_misses();
            if hits + misses > 0 {
                tracing::info!(hits, misses, "shared-surface import cache");
            }
        }
    }
}

/// Milliseconds since the Unix epoch, for the app's clock.
pub(crate) fn unix_ms() -> u64 {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    since_epoch.as_millis() as u64
}
