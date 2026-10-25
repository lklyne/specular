//! Headless runs: `--snapshot` and `--script` draw a canvas into PNG files
//! on the real adapter, with no window on screen.
//!
//! The run is a [`Runtime`] like a window's, over a [`HeadlessWindow`], so
//! every effect has the one runner the Kit window uses. A script's input
//! goes through a [`Driver`], the scripted hands the testkit uses too.
//! Pages come from the synthetic source, or with `--source cef` from CEF,
//! pumped here until they have loaded and painted. Nothing is written but
//! the PNGs and what a `save` step names: the runtime is told not to write,
//! so autosaves, assets and preferences are dropped, and the clipboard and
//! the Documents a run makes are kept in memory.

mod bench;
mod steps;
mod window;

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use specular_bench::PaintPolicy;
use specular_core::PageSource;
use specular_doc::{Document, Kind};
use specular_interact::{Driver, Event, Theme};

pub(crate) use self::bench::{BenchPlan, START_CAMERA, run as run_bench, work_of};
use self::window::HeadlessWindow;
use crate::app::{Runtime, RuntimeOptions};
use crate::cli::SourceKind;
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
/// with no panels, on the canvas its first line names (an empty one when it names none),
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
    run.drive(|app| {
        if let Some(theme) = args.theme {
            app.send(Event::ThemeLoaded(theme));
        }
        if let Some(folder) = run_folder(app.sink()) {
            app.send(Event::DocumentFolder(folder));
        }
        app.send(Event::DocumentOpened(Box::new(document)))
    })?;
    // A script's first step may ask a page something, and a real page has
    // nothing to say until it has loaded.
    if run.live {
        run.runtime().settle(true);
    }
    run.step(Step::Camera(args.camera))?;
    run.checks = Checks::begin(run.hands.app(), &base)?;
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
    run.runtime().shut_down();
    if !report.skipped.is_empty() {
        eprintln!(
            "skipped {} window-only steps:\n  {}",
            report.skipped.len(),
            report.skipped.join("\n  ")
        );
    }
    Ok(report)
}

/// The folder of the canvas a run shows, which its relative paths start
/// from.
fn run_folder(runtime: &Runtime<HeadlessWindow>) -> Option<String> {
    let folder = runtime.space_folder()?;
    Some(folder.to_string_lossy().into_owned())
}

struct Headless {
    /// The script's hands, sending to the runtime.
    hands: Driver<Runtime<HeadlessWindow>>,
    /// Whether the pages are real ones, which load and paint in their own
    /// time.
    live: bool,
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
        let window = HeadlessWindow::new(args.size, args.scale)?;
        let mut runtime = Runtime::new(
            source,
            RuntimeOptions {
                canvas: canvas.map(Path::to_owned),
                paint_policy: PaintPolicy::FullRate,
                chrome: true,
                settings: false,
                motion: false,
                writes: false,
                start_ms: Some(0),
            },
        );
        runtime.attach_window(window);
        runtime.clear_sent();
        Ok(Self {
            hands: Driver::over(runtime),
            live: args.source == SourceKind::Cef,
            now_ms: START_MS,
            out: out.to_owned(),
            checks: Checks::default(),
        })
    }

    fn runtime(&mut self) -> &mut Runtime<HeadlessWindow> {
        self.hands.sink_mut()
    }

    /// Scripts input into the app. The runtime runs each effect as it
    /// comes; one that fails ends the run.
    fn drive(
        &mut self,
        input: impl for<'hands> FnOnce(
            &'hands mut Driver<Runtime<HeadlessWindow>>,
        ) -> &'hands mut Driver<Runtime<HeadlessWindow>>,
    ) -> anyhow::Result<()> {
        input(&mut self.hands);
        self.runtime().take_error().map_or(Ok(()), Err)
    }

    /// Draws the app as it stands and writes the frame to `path`.
    fn snapshot(&mut self, path: &Path) -> anyhow::Result<()> {
        // Two ticks with the camera held still: svgs are drawn at the size
        // they are shown at once the zoom has stopped.
        let now = self.now_ms;
        self.drive(|app| app.tick(now).tick(now))?;
        let live = self.live;
        self.runtime().settle(live);
        let path = self.out.join(path);
        // The second frame is the settled one: text held at a zoom's old
        // size is put right by it.
        self.runtime().draw();
        self.runtime().draw();
        let window = (self.hands.sink().window()).context("the headless window is gone")?;
        window.save(&path)?;
        tracing::info!(path = %path.display(), "snapshot");
        Ok(())
    }

    /// Draws a frame when a Document is on the canvas: its rows are only
    /// measured when drawn, and their heights move what is below them.
    fn frame_for_documents(&mut self) -> anyhow::Result<()> {
        let documents = (self.hands.document().entities()).any(
            |entity| matches!(&entity.kind, Kind::File(file) if script::is_markdown(&file.file)),
        );
        if documents {
            self.runtime().draw();
        }
        self.runtime().take_error().map_or(Ok(()), Err)
    }

    /// Writes the `.canvas` text an autosave would write now to `path`.
    fn save(&self, path: &Path) -> anyhow::Result<()> {
        let path = self.out.join(path);
        let text = script::saved_text(self.hands.app())?;
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
    }
}
