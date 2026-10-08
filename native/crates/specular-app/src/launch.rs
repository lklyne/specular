//! What both shells do before a window opens: read the command line, start
//! logging, work out what to show, and run a headless request to its end.

use std::ffi::OsString;
use std::io::IsTerminal as _;
use std::path::PathBuf;

use specular_core::PageSource;
use specular_doc::Document;
use specular_interact::SpaceAsk;
use tracing_subscriber::EnvFilter;

use crate::app::{Opening, RuntimeOptions};
use crate::bench_drive::BenchOptions;
use crate::cli::{self, Command, RunArgs};
use crate::source_select::{self, Host};
use crate::space::{SpaceChoice, SpaceStart, Startup};
use crate::{headless, prefs, scene, space};

/// The pages' profile, in this app's data folder: cookies and logins.
const PROFILE_FOLDER: &str = "cef-profile";

/// A run that opens a window: what the command line asked for, and what to
/// show first.
#[derive(Debug)]
pub struct Launch {
    pub(crate) run: RunArgs,
    pub(crate) space: Option<SpaceStart>,
    /// Why no space opens, when the user is to be asked for one.
    ask: Option<SpaceAsk>,
    /// Where the pages' profile is kept, for a launch that keeps one.
    profile: Option<PathBuf>,
    pub(crate) document: Document,
}

/// What a launch that names no space opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unnamed {
    /// The scratch space: for a shell with no view to ask in.
    Scratch,
    /// The space chosen in this app, and the first-run view when there is
    /// none or it is gone.
    Chosen,
}

/// Reads `args` (without the program name) and starts logging. `None` when
/// there is no window to open: help was printed, or the run was a
/// `--snapshot` or `--script` and is finished. A wrong argument prints the
/// usage and exits the process.
pub fn launch(
    args: impl IntoIterator<Item = OsString>,
    unnamed: Unnamed,
) -> anyhow::Result<Option<Launch>> {
    let run = match cli::parse(args) {
        Ok(Command::Run(run)) => run,
        Ok(Command::Help) => {
            println!("{}", cli::USAGE);
            return Ok(None);
        }
        Err(error) => {
            eprintln!("{error:#}\n\n{}", cli::USAGE);
            std::process::exit(2);
        }
    };
    // Logs go to stderr so `--bench` output on stdout stays pure JSON lines.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(std::io::stderr().is_terminal())
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let choice = run.space_choice(match unnamed {
        Unnamed::Scratch => SpaceChoice::Scratch,
        Unnamed::Chosen => SpaceChoice::Chosen,
    });
    let (space, ask) = match space_to_open(&run, &choice) {
        Some(Startup::Open(space)) => (Some(space), None),
        Some(Startup::Ask(ask)) => (None, Some(ask)),
        None => (None, None),
    };
    // Logins belong to the app someone uses as theirs. A run on a fixture
    // or the scratch space keeps its profile in memory, so any number of
    // them run side by side.
    let own = matches!(choice, SpaceChoice::Chosen | SpaceChoice::User);
    let profile = (own && (space.is_some() || ask.is_some()))
        .then(prefs::folder)
        .flatten()
        .map(|folder| folder.join(PROFILE_FOLDER));
    let demo_pages = run.pages.unwrap_or(scene::DEMO_PAGE_COUNT);
    // With a space to open, the canvases are read once the window exists.
    // With one to ask for, nothing is shown behind the question.
    let document = if space.is_some() || ask.is_some() {
        Document::new()
    } else {
        scene::load_document(run.canvas.as_deref(), demo_pages, run.annotations)?
    };
    if let Some(profiles) = run.bench.clone().filter(|_| run.bench_headless) {
        let source = source_select::create_source(run.source, Host::Headless, None)?;
        let args = headless::HeadlessArgs {
            source: run.source,
            ..run.headless.clone()
        };
        let plan = headless::BenchPlan {
            profiles,
            start: match args.camera {
                headless::CameraArg::At(camera) => camera,
                headless::CameraArg::Fit => headless::START_CAMERA,
            },
            chrome: run.chrome,
        };
        headless::run_bench(source, document, run.canvas.as_deref(), &args, &plan)?;
        return Ok(None);
    }
    if run.headless.is_requested() {
        let source = source_select::create_source(run.headless.source, Host::Headless, None)?;
        headless::run(source, document, run.canvas.as_deref(), &run.headless)?;
        return Ok(None);
    }
    Ok(Some(Launch {
        run,
        space,
        ask,
        profile,
        document,
    }))
}

impl Launch {
    /// The page backend the command line chose. The platform's application
    /// object must exist first: CEF installs its own otherwise.
    pub fn create_source(&self) -> anyhow::Result<Box<dyn PageSource>> {
        source_select::create_source(self.run.source, Host::Window, self.profile.as_deref())
    }

    /// The window size asked for, in logical pixels.
    pub fn window_size(&self) -> Option<(u32, u32)> {
        self.run.window
    }

    /// The benchmark this launch runs in its window, when it is a
    /// `--bench` run. A shell starts it with [`Bench::start`](crate::Bench::start) once the
    /// window shows what [`into_opening`](Self::into_opening) gave.
    pub fn bench(&self) -> Option<BenchOptions> {
        let profiles = self.run.bench.clone()?;
        Some(BenchOptions {
            profiles,
            warmup: self.run.warmup,
            representative: self.run.source.is_representative(),
            annotations: self.run.annotations,
        })
    }

    /// How the runtime of this launch runs.
    pub fn runtime_options(&self) -> RuntimeOptions {
        RuntimeOptions {
            canvas: self.run.canvas.clone().filter(|_| self.space.is_none()),
            paint_policy: self.run.paint_policy,
            chrome: self.run.chrome,
            // A benchmark neither reads nor writes settings.
            settings: self.run.bench.is_none(),
        }
    }

    /// What to show first.
    pub fn into_opening(self) -> Opening {
        Opening {
            space: self.space,
            ask: self.ask,
            document: Some(self.document),
        }
    }
}

/// The space this run opens, or `None` for a run that shows one document
/// and writes nothing: a snapshot or a script, a benchmark, a demo grid, or
/// a canvas with seeded annotations.
fn space_to_open(run: &RunArgs, choice: &SpaceChoice) -> Option<Startup> {
    let one_document = run.headless.is_requested()
        || run.bench.is_some()
        || run.pages.is_some()
        || run.annotations > 0;
    if one_document {
        return None;
    }
    // The user's own space is looked for only when it was asked for.
    let (electron, remembered) = if matches!(choice, SpaceChoice::User | SpaceChoice::Chosen) {
        (
            space::electron_user_data(|name| std::env::var_os(name), cfg!(target_os = "macos"))
                .and_then(|user_data| space::electron_space(&user_data)),
            prefs::file().and_then(|path| prefs::load_space_path(&path)),
        )
    } else {
        (None, None)
    };
    let scratch = space::scratch_folder(prefs::folder().as_deref());
    Some(space::startup(choice, electron, remembered, scratch))
}
