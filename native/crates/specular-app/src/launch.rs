//! What both shells do before a window opens: read the command line, start
//! logging, work out what to show, and run a headless request to its end.

use std::ffi::OsString;
use std::io::IsTerminal as _;

use anyhow::Context as _;
use specular_core::PageSource;
use specular_doc::Document;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

use crate::app::{self, Opening, RuntimeOptions};
use crate::cli::{self, Command, RunArgs};
use crate::source_select::{self, Host};
use crate::space::SpaceStart;
use crate::{headless, prefs, scene, space};

/// A run that opens a window: what the command line asked for, and what to
/// show first.
#[derive(Debug)]
pub struct Launch {
    run: RunArgs,
    space: Option<SpaceStart>,
    document: Document,
}

/// Reads `args` (without the program name) and starts logging. `None` when
/// there is no window to open: help was printed, or the run was a
/// `--snapshot` or `--script` and is finished. A wrong argument prints the
/// usage and exits the process.
pub fn launch(args: impl IntoIterator<Item = OsString>) -> anyhow::Result<Option<Launch>> {
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

    let space = space_to_open(&run);
    let demo_pages = run.pages.unwrap_or(scene::DEMO_PAGE_COUNT);
    // With a space to open, the canvases are read once the window exists.
    let document = if space.is_some() {
        Document::new()
    } else {
        scene::load_document(run.canvas.as_deref(), demo_pages, run.annotations)?
    };
    if run.headless.is_requested() {
        let source = source_select::create_source(run.headless.source, Host::Headless)?;
        headless::run(source, document, run.canvas.as_deref(), &run.headless)?;
        return Ok(None);
    }
    Ok(Some(Launch {
        run,
        space,
        document,
    }))
}

impl Launch {
    /// The page backend the command line chose. The platform's application
    /// object must exist first: CEF installs its own otherwise.
    pub fn create_source(&self) -> anyhow::Result<Box<dyn PageSource>> {
        source_select::create_source(self.run.source, Host::Window)
    }

    /// The window size asked for, in logical pixels.
    pub fn window_size(&self) -> Option<(u32, u32)> {
        self.run.window
    }

    /// Whether this is a `--bench` run, which only the winit shell does.
    pub fn is_bench(&self) -> bool {
        self.run.bench.is_some()
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
            document: Some(self.document),
        }
    }
}

/// Opens the winit window on `launch` and runs until it closes.
pub fn run_window(launch: Launch) -> anyhow::Result<()> {
    let Launch {
        run,
        space,
        document,
    } = launch;
    // winit must create the macOS application object before CEF initializes,
    // or CEF installs its own and winit panics.
    let mut event_loop = EventLoop::<app::ShellEvent>::with_user_event();
    // The shell installs its own menu bar, except in a benchmark.
    #[cfg(target_os = "macos")]
    winit::platform::macos::EventLoopBuilderExtMacOS::with_default_menu(
        &mut event_loop,
        run.bench.is_some(),
    );
    let event_loop = event_loop.build().context("creating event loop")?;
    let source = source_select::create_source(run.source, Host::Window)?;
    tracing::info!(
        backend = source.name(),
        entities = document.entities().count(),
        paint_policy = run.paint_policy.name(),
        "starting"
    );

    let options = app::RunOptions {
        canvas: run.canvas.filter(|_| space.is_none()),
        space,
        bench: run.bench,
        warmup: run.warmup,
        representative_source: run.source.is_representative(),
        paint_policy: run.paint_policy,
        window: run.window,
        chrome: run.chrome,
        annotations: run.annotations,
    };
    let mut app = app::Shell::new(source, document, options, event_loop.create_proxy());
    event_loop.run_app(&mut app).context("running event loop")?;
    app.into_result()
}

/// The space this run opens, or `None` for a run that shows one document
/// and writes nothing: a snapshot or a script, a benchmark, a demo grid, or
/// a canvas with seeded annotations.
fn space_to_open(run: &RunArgs) -> Option<SpaceStart> {
    let one_document = run.headless.is_requested()
        || run.bench.is_some()
        || run.pages.is_some()
        || run.annotations > 0;
    if one_document {
        return None;
    }
    let choice = run.space_choice();
    // The user's own space is looked for only when it was asked for.
    let (electron, remembered) = if choice == space::SpaceChoice::User {
        (
            space::electron_user_data(|name| std::env::var_os(name), cfg!(target_os = "macos"))
                .and_then(|user_data| space::electron_space(&user_data)),
            prefs::file().and_then(|path| prefs::load_space_path(&path)),
        )
    } else {
        (None, None)
    };
    let scratch = space::scratch_folder(prefs::folder().as_deref());
    space::startup(&choice, electron, remembered, scratch)
}
