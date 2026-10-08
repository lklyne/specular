//! Headless runs: `--snapshot` and `--script` draw a canvas into PNG files
//! on the real adapter, with no window.
//!
//! The app is a [`TestApp`], the driver the feature tests script, with the
//! compositor's text measure in place of the fixed one. Pages come from the
//! synthetic source, or with `--source cef` from CEF, pumped here until they
//! have loaded and painted. Images and Documents come from the shell's own
//! loader threads. Nothing is written but the PNGs and what a `save` step names:
//! autosaves, assets and preferences are dropped, and the clipboard and the
//! Documents a session makes are kept in memory.

mod effects;
mod script;
mod stand_ins;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context as _;
use glam::Vec2;
use specular_compositor::{Compositor, DotGrid, FrameView, GpuContext};
use specular_core::{PageId, PageSource};
use specular_doc::{Document, EntityId};
use specular_interact::{Action, Event, ImageKey};
use specular_testkit::TestApp;

pub(crate) use self::script::CameraArg;
use self::script::Step;
use self::stand_ins::StandIns;
use crate::cli::SourceKind;
use crate::images::ImageLoader;
use crate::notes::NoteLoader;
use crate::offscreen::{self, Target};
use crate::page_queries::PageQueries;
use crate::persist::canvas_text;

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
        }
    }
}

impl HeadlessArgs {
    /// Whether the run draws to files instead of opening a window.
    pub(crate) fn is_requested(&self) -> bool {
        self.snapshot.is_some() || self.script.is_some()
    }
}

/// `--snapshot-camera`'s value.
pub(crate) fn camera_arg(value: &str) -> anyhow::Result<CameraArg> {
    script::camera(value)
}

/// Opens `document` (read from `canvas`, if from a file), runs the script
/// and writes the snapshots.
pub(crate) fn run(
    source: Box<dyn PageSource>,
    document: Document,
    canvas: Option<&Path>,
    args: &HeadlessArgs,
) -> anyhow::Result<()> {
    let steps = match &args.script {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?;
            script::parse(&text).with_context(|| format!("in {}", path.display()))?
        }
        None => Vec::new(),
    };
    let mut run = Headless::new(source, canvas, args)?;
    let viewport = run.viewport;
    run.drive(|app| app.viewport(viewport).open(document))?;
    // A script's first step may ask a page something, and a real page has
    // nothing to say until it has loaded.
    if run.live {
        run.settle()?;
    }
    run.step(Step::Camera(args.camera))?;
    for step in steps {
        run.step(step)?;
    }
    if let Some(path) = &args.snapshot {
        run.snapshot(path)?;
    }
    run.source.shutdown();
    Ok(())
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
    app: TestApp,
    /// The hosted page behind each page entity.
    hosts: HashMap<EntityId, PageId>,
    /// What pages have been asked and not yet answered.
    queries: PageQueries,
    images: ImageLoader,
    notes: NoteLoader,
    /// Asked for and not yet answered.
    loading_images: HashSet<ImageKey>,
    loading_notes: HashSet<String>,
    stand_ins: StandIns,
    /// The row heights last reported for each Document on screen.
    note_heights: HashMap<EntityId, f32>,
    viewport: Vec2,
    scale: f32,
    now_ms: u64,
}

impl Headless {
    fn new(
        source: Box<dyn PageSource>,
        canvas: Option<&Path>,
        args: &HeadlessArgs,
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
        let mut app = TestApp::empty();
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
            notes: NoteLoader::new(space).context("starting the note thread")?,
            loading_images: HashSet::new(),
            loading_notes: HashSet::new(),
            stand_ins: StandIns::default(),
            note_heights: HashMap::new(),
            viewport: Vec2::new(width as f32, height as f32),
            scale: args.scale,
            now_ms: START_MS,
        })
    }

    /// Scripts input into the app and runs the effects that come back.
    fn drive(
        &mut self,
        input: impl for<'app> FnOnce(&'app mut TestApp) -> &'app mut TestApp,
    ) -> anyhow::Result<()> {
        input(&mut self.app);
        for effect in self.app.take_effects() {
            self.run(effect)?;
        }
        Ok(())
    }

    fn step(&mut self, step: Step) -> anyhow::Result<()> {
        match step {
            Step::Move(at) => self.drive(|app| app.pointer_move(at)),
            Step::Press(at) => self.drive(|app| app.press(at)),
            Step::DragTo(at) => self.drive(|app| app.drag_to(at)),
            Step::Release => self.drive(|app| app.release()),
            Step::Click(at) => self.drive(|app| app.pointer_move(at).click(at)),
            Step::DoubleClick(at) => self.drive(|app| app.pointer_move(at).double_click(at)),
            Step::TripleClick(at) => self.drive(|app| app.pointer_move(at).triple_click(at)),
            Step::Drag(from, to) => self.drive(|app| {
                app.pointer_move(from)
                    .press(from)
                    .drag_to(from.midpoint(to))
                    .drag_to(to)
                    .release()
            }),
            Step::Hold(modifiers) => self.drive(|app| app.hold(modifiers)),
            Step::Key(modifiers, key) => self.drive(|app| app.chord(modifiers, key)),
            Step::Type(text) => self.drive(|app| app.type_text(&text)),
            Step::Compose(text) => self.drive(|app| app.compose(&text)),
            Step::Commit(text) => self.drive(|app| app.commit(&text)),
            Step::Clipboard(text) => {
                self.stand_ins.clipboard = Some(text);
                Ok(())
            }
            Step::Wheel(delta) => self.drive(|app| app.wheel(delta)),
            Step::Pinch(delta) => self.drive(|app| app.pinch(delta)),
            Step::Tool(tool) => self.drive(|app| app.tool(tool)),
            Step::Act(action) => self.drive(|app| app.act(action)),
            Step::Select(ids) => self.drive(|app| {
                let ids: Vec<&str> = ids.iter().map(String::as_str).collect();
                app.select(&ids)
            }),
            Step::Camera(CameraArg::Fit) => {
                // The fit is of the document, which may still be growing as
                // text is measured; loads do not move it.
                self.drive(|app| app.act(Action::ZoomToFit))
            }
            Step::Camera(CameraArg::At(camera)) => {
                self.drive(|app| app.act(Action::SetCamera(camera)))
            }
            Step::Wait(ms) => {
                self.now_ms += ms;
                let now = self.now_ms;
                self.drive(|app| app.tick(now))?;
                self.run_pages_for(std::time::Duration::from_millis(ms))?;
                self.settle()
            }
            Step::Snapshot(path) => self.snapshot(&path),
            Step::Save(path) => self.save(&path),
        }
    }

    /// Draws the app as it stands and writes the frame to `path`.
    fn snapshot(&mut self, path: &Path) -> anyhow::Result<()> {
        self.settle()?;
        let scene = specular_scene::view(self.app.app(), self.viewport);
        let frame = FrameView {
            camera: self.app.session().camera,
            viewport: self.viewport,
            scale_factor: self.scale,
            grid: DotGrid::default(),
            zooming: false,
        };
        let hosts = &self.hosts;
        let stats = self
            .compositor
            .render_scene(&self.target.view(), &frame, &scene, |entity| {
                hosts.get(entity).copied()
            });
        self.target.save(&self.gpu, path)?;
        tracing::info!(path = %path.display(), items = scene.items.len(), ?stats, "snapshot");
        self.report_note_heights()
    }

    /// Tells the app how tall each Document's rows came out in the frame
    /// just drawn, as the shell does after every frame.
    fn report_note_heights(&mut self) -> anyhow::Result<()> {
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
        let text = canvas_text(
            &self.app.app().document_to_save(),
            self.app.session().camera,
        )
        .with_context(|| format!("writing {}", path.display()))?;
        std::fs::write(path, text).with_context(|| format!("writing {}", path.display()))
    }
}
