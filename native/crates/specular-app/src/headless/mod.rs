//! Headless runs: `--snapshot` and `--script` draw a canvas into PNG files
//! on the real adapter, with no window.
//!
//! The app is a [`TestApp`], the driver the feature tests script, with the
//! compositor's text measure in place of the fixed one. Pages come from the
//! synthetic source, and images and Documents from the shell's own loader
//! threads. Nothing is written but the PNGs: saves, the clipboard, assets
//! and preferences are dropped.

mod script;
mod target;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use glam::Vec2;
use specular_compositor::{Compositor, DotGrid, FrameView, GpuContext};
use specular_core::{PageEvent, PageId, PageSource, PageSpec, SyntheticPageSource};
use specular_doc::{Document, EntityId};
use specular_interact::{Action, Effect, Event, ImageKey, ImageNotice, NoteNotice, PageNotice};
use specular_scene::ImageId;
use specular_testkit::TestApp;

pub(crate) use self::script::CameraArg;
use self::script::Step;
use self::target::Target;
use crate::images::{ImageLoader, LoadFailure};
use crate::notes::{NoteLoader, ReadFailure};

/// How long a snapshot waits for images and Documents before drawing
/// without them.
const LOAD_TIMEOUT: Duration = Duration::from_secs(10);
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
}

impl Default for HeadlessArgs {
    fn default() -> Self {
        Self {
            snapshot: None,
            size: (1600, 1000),
            scale: 1.0,
            camera: CameraArg::Fit,
            script: None,
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
    let mut run = Headless::new(canvas, args)?;
    let viewport = run.viewport;
    run.drive(|app| app.viewport(viewport).open(document))?;
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
    source: SyntheticPageSource,
    app: TestApp,
    /// The hosted page behind each page entity.
    hosts: HashMap<EntityId, PageId>,
    images: ImageLoader,
    notes: NoteLoader,
    /// Asked for and not yet answered.
    loading_images: HashSet<ImageKey>,
    loading_notes: HashSet<String>,
    viewport: Vec2,
    scale: f32,
    now_ms: u64,
}

impl Headless {
    fn new(canvas: Option<&Path>, args: &HeadlessArgs) -> anyhow::Result<Self> {
        let gpu = pollster::block_on(GpuContext::headless()).context("no GPU to draw with")?;
        let mut compositor = Compositor::new(gpu.device.clone(), gpu.queue.clone(), target::FORMAT);
        compositor.warm_text();
        let (width, height) = args.size;
        let pixels = |side: u32| ((side as f32 * args.scale).round() as u32).max(1);
        let target = Target::new(&gpu, pixels(width), pixels(height));
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
            source: SyntheticPageSource::new(),
            app,
            hosts: HashMap::new(),
            images: ImageLoader::new(space.clone()).context("starting the decode thread")?,
            notes: NoteLoader::new(space).context("starting the note thread")?,
            loading_images: HashSet::new(),
            loading_notes: HashSet::new(),
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
            Step::Tool(tool) => self.drive(|app| app.tool(tool)),
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
                self.settle()
            }
            Step::Snapshot(path) => self.snapshot(&path),
        }
    }

    fn run(&mut self, effect: Effect) -> anyhow::Result<()> {
        match effect {
            Effect::CreatePage {
                page,
                url,
                viewport,
            } => {
                let mut spec = PageSpec::new(&url, viewport);
                spec.texture_scale = self.scale;
                let host = self
                    .source
                    .create_page(&spec)
                    .with_context(|| format!("creating page for {url}"))?;
                self.hosts.insert(page, host);
            }
            Effect::ClosePage(page) => {
                if let Some(host) = self.hosts.remove(&page) {
                    self.source.close_page(host)?;
                    self.compositor.remove_page(host);
                }
            }
            Effect::SetPageViewport { page, viewport } => {
                if let Some(&host) = self.hosts.get(&page) {
                    self.source.set_viewport(host, viewport)?;
                }
            }
            Effect::LoadImage { image, file } => {
                self.loading_images.insert(image);
                self.images
                    .request(image, &file, self.compositor.image_spec());
            }
            Effect::DropImage(image) => {
                self.loading_images.remove(&image);
                self.compositor.remove_image(ImageId(image.0));
            }
            Effect::LoadNote { file } => {
                self.notes.watch(&file);
                self.loading_notes.insert(file);
            }
            Effect::DropNote { file } => {
                self.notes.unwatch(&file);
                self.loading_notes.remove(&file);
            }
            // A headless run has no window to focus, no cursor and no input
            // method, and it leaves the disk and the clipboard alone.
            Effect::FocusPage(_)
            | Effect::ForwardInput { .. }
            | Effect::SetImeAllowed(_)
            | Effect::SetImeCursorArea { .. }
            | Effect::SetCursor(_)
            | Effect::Save
            | Effect::WriteClipboard(_)
            | Effect::ReadClipboard
            | Effect::WriteAsset { .. }
            | Effect::CopyAsset { .. }
            | Effect::WriteNote { .. }
            | Effect::CreateNote { .. }
            | Effect::SaveToolDefaults(_) => {}
        }
        Ok(())
    }

    /// Takes a frame from every page and waits for the images and Documents
    /// asked for so far.
    fn settle(&mut self) -> anyhow::Result<()> {
        let deadline = Instant::now() + LOAD_TIMEOUT;
        loop {
            self.take_page_events()?;
            self.take_images()?;
            self.take_notes()?;
            if self.loading_images.is_empty() && self.loading_notes.is_empty() {
                return Ok(());
            }
            if Instant::now() >= deadline {
                tracing::warn!(
                    images = self.loading_images.len(),
                    documents = self.loading_notes.len(),
                    "drawing without loads that never finished"
                );
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn take_page_events(&mut self) -> anyhow::Result<()> {
        let mut events = Vec::new();
        self.source.pump();
        self.source.drain_events(&mut events);
        for event in events {
            if let PageEvent::Loaded { page, http_status } = &event
                && let Some(entity) = self.entity_of(*page)
            {
                let notice = PageNotice::Loaded {
                    http_status: *http_status,
                };
                self.drive(|app| {
                    app.send(Event::Page {
                        page: entity,
                        notice,
                    })
                })?;
            }
            self.compositor.handle_page_event(event)?;
        }
        Ok(())
    }

    fn entity_of(&self, page: PageId) -> Option<EntityId> {
        let (entity, _) = self.hosts.iter().find(|&(_, &host)| host == page)?;
        Some(entity.clone())
    }

    fn take_images(&mut self) -> anyhow::Result<()> {
        while let Some(loaded) = self.images.take() {
            if !self.loading_images.remove(&loaded.key) {
                continue;
            }
            let notice = match loaded.result {
                Ok(mips) => {
                    self.compositor
                        .set_image_mips(ImageId(loaded.key.0), &mips)?;
                    ImageNotice::Ready {
                        width: mips.size().width,
                        height: mips.size().height,
                    }
                }
                Err(LoadFailure::Missing) => ImageNotice::Missing,
                Err(LoadFailure::Failed) => ImageNotice::Failed,
            };
            let image = loaded.key;
            self.drive(|app| app.send(Event::Image { image, notice }))?;
        }
        Ok(())
    }

    fn take_notes(&mut self) -> anyhow::Result<()> {
        while let Some(read) = self.notes.take() {
            self.loading_notes.remove(&read.file);
            let notice = match read.result {
                Ok(text) => NoteNotice::Text(text),
                Err(ReadFailure::Missing) => NoteNotice::Missing,
                Err(ReadFailure::Failed) => NoteNotice::Failed,
            };
            let file = read.file;
            self.drive(|app| app.send(Event::Note { file, notice }))?;
        }
        Ok(())
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
        Ok(())
    }
}
