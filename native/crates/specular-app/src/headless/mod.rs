//! Headless runs: `--snapshot` and `--script` draw a canvas into PNG files
//! on the real adapter, with no window.
//!
//! The app is a [`Driver`], the scripted hands the feature tests use too,
//! with the compositor's text measure. Pages come from the
//! synthetic source, or with `--source cef` from CEF, pumped here until they
//! have loaded and painted. Images and Documents come from the shell's own
//! loader threads. Nothing is written but the PNGs and what a `save` step names:
//! autosaves, assets and preferences are dropped, and the clipboard and the
//! Documents a session makes are kept in memory.

mod bench;
mod effects;
mod stand_ins;
mod steps;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context as _;
use glam::Vec2;
use specular_compositor::{Compositor, DotGrid, FrameView, GpuContext};
use specular_core::{PageId, PageSource};
use specular_doc::{Document, EntityId, Kind};
use specular_interact::{App, Driver, Event, ImageKey, Theme};

pub(crate) use self::bench::{BenchPlan, START_CAMERA, run as run_bench, work_of};
use self::stand_ins::StandIns;
use crate::cli::SourceKind;
use crate::images::{ImageLoader, Uploaded};
use crate::notes::NoteLoader;
use crate::offscreen::{self, Target};
use crate::page_queries::PageQueries;
pub(crate) use crate::script::CameraArg;
use crate::script::{self, Checks, Script, Step};

/// The clock a run starts at, so two runs of one script draw the same frame.
const START_MS: u64 = 1_700_000_000_000;

/// What the command line asked a headless run to do.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HeadlessArgs {
    /// The PNG written after the script, if any.
    pub(crate) snapshot: Option<PathBuf>,
    /// The viewport in logical pixels.
    pub(crate) size: (u32, u32),
    /// Device pixels per logical pixel.
    pub(crate) scale: f32,
    /// The camera the run starts at.
    pub(crate) camera: CameraArg,
    /// The script run before the snapshot, if any.
    pub(crate) script: Option<PathBuf>,
    /// The page backend. Synthetic unless the command line names another.
    pub(crate) source: SourceKind,
    /// Whether the built-in panels are on when the script starts.
    pub(crate) panels: bool,
    /// The theme the run starts in. `None` is the system's, which a
    /// headless run reports as light.
    pub(crate) theme: Option<Theme>,
}

impl Default for HeadlessArgs {
    fn default() -> Self {
        Self {
            snapshot: None,
            size: (1600, 1000),
            scale: 1.0,
            camera: CameraArg::Fit,
            script: None,
            source: SourceKind::Synthetic,
            panels: true,
            theme: None,
        }
    }
}

impl HeadlessArgs {
    /// Whether the run draws to files instead of opening a window.
    pub(crate) fn is_requested(&self) -> bool {
        self.snapshot.is_some() || self.script.is_some()
    }
}

/// The theme a `--theme` value names.
pub(crate) fn theme_arg(value: &str) -> anyhow::Result<Theme> {
    script::theme_named(value)
}

/// `--snapshot-camera`'s value.
pub(crate) fn camera_arg(value: &str) -> anyhow::Result<CameraArg> {
    script::camera(value)
}

/// What a finished script run did besides pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// The steps only a window can do, which this run skipped, as
    /// `line N: text`.
    pub skipped: Vec<String>,
}

/// The canvas the script at `script` names on its first line, relative to
/// the script's folder.
pub(crate) fn script_canvas(script: &Path) -> anyhow::Result<Option<PathBuf>> {
    let folder = script.parent().unwrap_or(Path::new(""));
    Ok(Script::read(script)?
        .canvas
        .map(|canvas| folder.join(canvas)))
}

/// Runs the scenario script at `script` headless on the synthetic pages,
/// on the canvas its first line names (an empty one when it names none),
/// and writes what it saves and draws into `out`. Fails at the first step
/// that fails or fact that does not hold.
pub fn run_scenario(script: &Path, out: &Path) -> anyhow::Result<Report> {
    let canvas = script_canvas(script)?;
    let document = match &canvas {
        Some(canvas) => crate::scene::load_document(Some(canvas), 0, 0)?,
        None => Document::new(),
    };
    let source = crate::source_select::create_source(
        SourceKind::Synthetic,
        crate::source_select::Host::Headless,
        None,
    )?;
    let args = HeadlessArgs {
        script: Some(script.to_owned()),
        ..HeadlessArgs::default()
    };
    run(source, document, canvas.as_deref(), &args, out)
}

/// Opens `document` (read from `canvas`, if from a file), runs the script
/// and writes the snapshots. Paths in the script are relative to `out`.
pub(crate) fn run(
    source: Box<dyn PageSource>,
    document: Document,
    canvas: Option<&Path>,
    args: &HeadlessArgs,
    out: &Path,
) -> anyhow::Result<Report> {
    let script = match &args.script {
        Some(path) => Script::read(path)?,
        None => Script::default(),
    };
    let base = (args.script.as_deref())
        .and_then(Path::parent)
        .map_or_else(PathBuf::new, Path::to_owned);
    let document = script.given_document()?.unwrap_or(document);
    let mut run = Headless::new(source, canvas, args, out)?;
    let (viewport, panels) = (run.viewport, args.panels);
    let folder = (run.space.as_deref()).map(|folder| folder.to_string_lossy().into_owned());
    run.drive(|app| {
        app.viewport(viewport).send(Event::BuiltinPanels(panels));
        if let Some(theme) = args.theme {
            app.send(Event::ThemeLoaded(theme));
        }
        if let Some(folder) = folder {
            app.send(Event::DocumentFolder(folder));
        }
        app.send(Event::DocumentOpened(Box::new(document)))
    })?;
    // A script's first step may ask a page something, and a real page has
    // nothing to say until it has loaded.
    if run.live {
        run.settle()?;
    }
    run.step(Step::Camera(args.camera))?;
    run.checks = Checks::begin(run.app.app(), &base)?;
    let mut report = Report::default();
    for line in script.steps {
        if line.step.window_only() {
            report
                .skipped
                .push(format!("line {}: {}", line.number, line.text));
            continue;
        }
        run.step(line.step)
            .with_context(|| format!("line {}: `{}`", line.number, line.text))?;
    }
    if let Some(path) = &args.snapshot {
        run.snapshot(path)?;
    }
    run.source.shutdown();
    if !report.skipped.is_empty() {
        eprintln!(
            "skipped {} window-only steps:\n  {}",
            report.skipped.len(),
            report.skipped.join("\n  ")
        );
    }
    Ok(report)
}

struct Headless {
    gpu: GpuContext,
    compositor: Compositor,
    target: Target,
    source: Box<dyn PageSource>,
    /// Whether the pages are real ones, which load and paint in their own
    /// time.
    live: bool,
    /// The pages whose current document has not finished loading.
    loading_pages: HashSet<PageId>,
    /// The pages that have not painted since their document loaded.
    unpainted_pages: HashSet<PageId>,
    app: Driver,
    /// The hosted page behind each page entity.
    hosts: HashMap<EntityId, PageId>,
    /// What pages have been asked and not yet answered.
    queries: PageQueries,
    images: ImageLoader,
    /// The folder the document's relative file paths start from.
    space: Option<PathBuf>,
    uploaded: Uploaded,
    notes: NoteLoader,
    /// Asked for and not yet answered.
    loading_images: HashSet<ImageKey>,
    loading_notes: HashSet<String>,
    stand_ins: StandIns,
    /// The row heights last reported for each Document on screen.
    note_heights: HashMap<EntityId, f32>,
    /// What `view` keeps from one frame to the next.
    view_cache: specular_scene::ViewCache,
    viewport: Vec2,
    scale: f32,
    now_ms: u64,
    /// The folder a script's relative paths are written in.
    out: PathBuf,
    /// What the `expect` steps compare with.
    checks: Checks,
}

impl Headless {
    fn new(
        source: Box<dyn PageSource>,
        canvas: Option<&Path>,
        args: &HeadlessArgs,
        out: &Path,
    ) -> anyhow::Result<Self> {
        let gpu = pollster::block_on(GpuContext::headless()).context("no GPU to draw with")?;
        let mut compositor =
            Compositor::new(gpu.device.clone(), gpu.queue.clone(), offscreen::FORMAT);
        compositor.warm_text();
        let (width, height) = args.size;
        let pixels = |side: u32| ((side as f32 * args.scale).round() as u32).max(1);
        let target = Target::new(&gpu, pixels(width), pixels(height), offscreen::FORMAT);
        let space = canvas
            .and_then(|canvas| std::path::absolute(canvas).ok())
            .and_then(|canvas| canvas.parent().map(Path::to_owned));
        let mut app = Driver::new(App::new(0));
        app.measure_with(Arc::new(compositor.text_measure()));
        tracing::info!(adapter = %gpu.adapter.get_info().name, "drawing headless");
        Ok(Self {
            gpu,
            compositor,
            target,
            source,
            live: args.source == SourceKind::Cef,
            loading_pages: HashSet::new(),
            unpainted_pages: HashSet::new(),
            app,
            hosts: HashMap::new(),
            queries: PageQueries::default(),
            images: ImageLoader::new(space.clone()).context("starting the decode thread")?,
            uploaded: Uploaded::default(),
            notes: NoteLoader::new(space.clone()).context("starting the note thread")?,
            space,
            loading_images: HashSet::new(),
            loading_notes: HashSet::new(),
            stand_ins: StandIns::default(),
            note_heights: HashMap::new(),
            view_cache: specular_scene::ViewCache::default(),
            viewport: Vec2::new(width as f32, height as f32),
            scale: args.scale,
            now_ms: START_MS,
            out: out.to_owned(),
            checks: Checks::default(),
        })
    }

    /// Scripts input into the app and runs the effects that come back.
    fn drive(
        &mut self,
        input: impl for<'app> FnOnce(&'app mut Driver) -> &'app mut Driver,
    ) -> anyhow::Result<()> {
        input(&mut self.app);
        for effect in self.app.take_effects() {
            self.run(effect)?;
        }
        Ok(())
    }

    /// Draws the app as it stands and writes the frame to `path`.
    fn snapshot(&mut self, path: &Path) -> anyhow::Result<()> {
        // Two ticks with the camera held still: svgs are drawn at the size
        // they are shown at once the zoom has stopped.
        let now = self.now_ms;
        self.drive(|app| app.tick(now).tick(now))?;
        self.settle()?;
        let path = self.out.join(path);
        let items = self.draw(true)?;
        self.target.save(&self.gpu, &path)?;
        tracing::info!(path = %path.display(), items, "snapshot");
        self.report_note_heights()
    }

    /// Draws a frame, with the built-in panels when `panels`, as a window
    /// would after a step, and returns how many items it drew.
    fn draw(&mut self, panels: bool) -> anyhow::Result<usize> {
        let mut scene = specular_scene::view(self.app.app(), self.viewport, &self.view_cache);
        if panels {
            specular_scene::draw_panels(self.app.app(), &mut scene);
        }
        let frame = FrameView {
            camera: self.app.session().camera,
            viewport: self.viewport,
            scale_factor: self.scale,
            grid: DotGrid::themed(scene.colors()),
            zooming: false,
        };
        let hosts = &self.hosts;
        let stats = self
            .compositor
            .render_scene(&self.target.view(), &frame, &scene, |entity| {
                hosts.get(entity).copied()
            });
        tracing::debug!(?stats, "frame");
        Ok(scene.items.len())
    }

    /// Draws a frame when a Document is on the canvas: its rows are only
    /// measured when drawn, and their heights move what is below them.
    fn frame_for_documents(&mut self) -> anyhow::Result<()> {
        let documents = (self.app.document().entities())
            .any(|entity| matches!(&entity.kind, Kind::File(file) if file.file.ends_with(".md")));
        if documents {
            self.draw(false)?;
            self.report_note_heights()?;
        }
        Ok(())
    }

    /// Tells the app how tall each Document's rows came out in the frame
    /// just drawn, as the shell does after every frame.
    pub(super) fn report_note_heights(&mut self) -> anyhow::Result<()> {
        let changed: Vec<_> = (self.compositor.column_heights().iter())
            .filter(|(entity, height)| self.note_heights.get(entity) != Some(height))
            .cloned()
            .collect();
        if changed.is_empty() {
            return Ok(());
        }
        self.note_heights.extend(changed.iter().cloned());
        self.drive(|app| app.send(Event::NoteHeights(changed)))
    }

    /// Writes the `.canvas` text an autosave would write now to `path`.
    fn save(&self, path: &Path) -> anyhow::Result<()> {
        let path = self.out.join(path);
        let text = script::saved_text(self.app.app())?;
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
    }
}
