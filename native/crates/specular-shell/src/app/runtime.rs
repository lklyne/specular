//! The half of a shell that needs no window toolkit: the [`App`], the page
//! backend, the files of the open space, and the runner of every
//! [`Effect`](specular_interact::Effect).
//!
//! A shell owns one [`Runtime`], gives it a [`ShellWindow`] to draw into,
//! and sends it [`Event`]s. `update` is the only thing that changes the
//! app, and [`Runtime::dispatch`] is the only caller of `update`.

mod window;

pub use self::window::{PageOf, ShellWindow};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use glam::Vec2;
use specular_bench::{FrameWork, PaintPolicy};
use specular_core::{PageEvent, PageId, PageSource};
use specular_doc::{Document, EntityId};
use specular_interact::{Action, ApiOutcome, App, Event, ImageKey, SpaceAsk, update};

use super::demand::FrameDemand;
use super::{START_CAMERA, agent_run, image_run, note_run, repos_run};
use crate::agent::AgentRuns;
use crate::api::ApiHost;
use crate::cdp::CdpHost;
use crate::images::{ImageLoader, Uploaded};
use crate::latency::InputLatencyProbe;
use crate::notes::NoteLoader;
use crate::page_files::PageFiles;
use crate::page_queries::PageQueries;
use crate::paint_lod::PageLod;
use crate::prefs;
use crate::space::{SpaceFiles, SpaceStart};

/// How a [`Runtime`] runs, from the command line.
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent switches: a benchmark, a headless run and a window each set them differently"
)]
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
    /// Whether the camera moves by a spring between the canvas, Device and
    /// Fill. Not in a benchmark, which needs the camera to land at once.
    pub(crate) motion: bool,
    /// Whether a thread's send starts a real agent run. Not headless, in a
    /// benchmark or in a scripted window run, where the script plays the
    /// agent's side.
    pub(crate) agents: bool,
    /// Whether effects may write to disk or ask the user: assets, Documents,
    /// threads, folder dialogs. Off headless, where the Documents a run
    /// makes and the clipboard are kept in memory.
    pub(crate) writes: bool,
    /// The clock the app starts at, for a run that must draw the same frame
    /// each time. `None` is the wall clock.
    pub(crate) start_ms: Option<u64>,
}

impl RuntimeOptions {
    /// The options for a window a script drives: the script plays the
    /// agent's side, so no real agent runs.
    #[must_use]
    pub fn scripted(self) -> Self {
        Self {
            agents: false,
            ..self
        }
    }
}

/// What a shell shows first: a space folder, or one document that is saved
/// nowhere.
#[derive(Debug)]
pub struct Opening {
    pub(crate) space: Option<SpaceStart>,
    /// Why no space opens, when the user is to be asked for one.
    pub(crate) ask: Option<SpaceAsk>,
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
    /// The HTML files the hosted pages load, watched for changes.
    pub(crate) page_files: PageFiles,
    /// The hosted page behind each page entity.
    pub(crate) hosts: HashMap<EntityId, PageHost>,
    /// What pages have been asked and not yet answered.
    pub(crate) queries: PageQueries,
    /// The decode thread. `None` if it could not be started; every image
    /// then stays a placeholder.
    pub(crate) image_loader: Option<ImageLoader>,
    /// The images the app has asked for and not let go of.
    pub(crate) images: HashSet<ImageKey>,
    /// The textures those images hold on the GPU.
    pub(crate) uploaded: Uploaded,
    /// The thread that reads and watches markdown files. `None` if it could
    /// not be started; every Document then stays on its loading line.
    pub(crate) note_loader: Option<NoteLoader>,
    /// The Document heights the app was last told, to tell it only changes.
    pub(crate) note_heights: HashMap<EntityId, f32>,
    /// What `view` keeps from one frame to the next.
    pub(crate) view_cache: specular_scene::ViewCache,
    /// The live room connection of each shared canvas.
    pub(crate) shares: super::share_run::Shares,
    /// The `claude` runs in flight, one a thread.
    pub(crate) agents: AgentRuns,
    /// The system clipboard, once something has been copied or pasted.
    pub(crate) clipboard: Option<arboard::Clipboard>,
    /// The clipboard of a scripted run, which leaves the system's alone:
    /// what was last copied, by the app or by the script.
    pub(crate) scripted_clipboard: Option<super::scripted::HeldClipboard>,
    /// The Documents a run that does not write makes, kept in memory.
    pub(crate) held_notes: Option<super::held_notes::HeldNotes>,
    /// What is asked for and not yet in: what a headless run waits on.
    pub(crate) loads: super::loads::Loads,
    /// The effects `update` returned since this was last taken, while a
    /// script reads them.
    pub(crate) sent: Option<Vec<specular_interact::Effect>>,
    /// The preferences file. `None` when settings are off, and when there
    /// is no home folder to keep it in.
    pub(crate) prefs: Option<PathBuf>,
    /// The repos file. `None` when settings are off, and when no folder
    /// can be found for it.
    pub(crate) repos_file: Option<PathBuf>,
    /// A folder dialog the app asked for and the window has yet to show.
    pub(crate) folder_dialog: Option<super::FolderAsk>,
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
        let mut app = App::new(options.start_ms.unwrap_or_else(unix_ms));
        update(&mut app, Event::Motion(options.motion));
        Self {
            source,
            space: space_folder.clone(),
            files: None,
            scratch: None,
            app,
            page_files: PageFiles::default(),
            hosts: HashMap::new(),
            queries: PageQueries::default(),
            image_loader: image_run::start_loader(space_folder.clone()),
            images: HashSet::new(),
            uploaded: Uploaded::default(),
            note_loader: note_run::start_loader(space_folder),
            note_heights: HashMap::new(),
            view_cache: specular_scene::ViewCache::default(),
            shares: super::share_run::Shares::default(),
            agents: agent_run::start_runs(options.agents),
            clipboard: None,
            scripted_clipboard: (!options.writes).then(super::scripted::HeldClipboard::default),
            held_notes: (!options.writes).then(super::held_notes::HeldNotes::default),
            loads: super::loads::Loads::default(),
            sent: None,
            prefs,
            repos_file: options.settings.then(repos_run::file).flatten(),
            folder_dialog: None,
            dropped: Vec::new(),
            dropped_at: None,
            api: None,
            cdp: None,
            api_outcome: None,
            title: String::new(),
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
        self.load_settings();
        self.load_theme();
        self.load_models();
        self.load_repos();
        if let Some(ask) = opening.ask {
            self.dispatch(Event::SpaceNeeded(ask));
        }
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
            (None, Some(document)) => {
                if let Some(folder) = self.space.as_deref() {
                    let folder = folder.to_string_lossy().into_owned();
                    self.dispatch(Event::DocumentFolder(folder));
                }
                self.dispatch(Event::DocumentOpened(Box::new(document)));
            }
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
        self.page_files = PageFiles::default();
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
