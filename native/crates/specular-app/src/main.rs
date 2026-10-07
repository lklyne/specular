//! `specular-app`: the winit shell for the Rust CEF spike.
//!
//! Hosts pages from a `.canvas` file (or a demo grid) on a pannable, zoomable
//! canvas, in CEF (`--source cef`, needs `--features cef`) or the synthetic
//! source (non-representative CPU frames). `--bench` replays the Electron
//! pan/zoom profiles and prints frame timing; see `cli::USAGE`.

mod app;
mod bench_run;
mod cli;
mod latency;
mod paint_lod;
mod scene;
mod source_select;
mod translate;

use std::io::IsTerminal as _;

use anyhow::Context as _;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

use crate::cli::Command;

fn main() -> anyhow::Result<()> {
    if let Some(code) = source_select::run_subprocess_if_needed() {
        std::process::exit(code);
    }
    let run = match cli::parse(std::env::args_os().skip(1)) {
        Ok(Command::Run(run)) => run,
        Ok(Command::Help) => {
            println!("{}", cli::USAGE);
            return Ok(());
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

    let demo_pages = run.pages.unwrap_or(scene::DEMO_PAGE_COUNT);
    let document = scene::load_document(run.canvas.as_deref(), demo_pages, run.annotations)?;
    // winit must create the macOS application object before CEF initializes,
    // or CEF installs its own and winit panics.
    let event_loop = EventLoop::new().context("creating event loop")?;
    let source = source_select::create_source(run.source)?;
    tracing::info!(
        backend = source.name(),
        pages = document.entities().count(),
        paint_policy = run.paint_policy.name(),
        "starting"
    );

    let options = app::RunOptions {
        bench: run.bench,
        warmup: run.warmup,
        representative_source: run.source.is_representative(),
        paint_policy: run.paint_policy,
        window: run.window,
        chrome: run.chrome,
        annotations: run.annotations,
    };
    let mut app = app::Shell::new(source, document, options);
    event_loop.run_app(&mut app).context("running event loop")?;
    app.into_result()
}
