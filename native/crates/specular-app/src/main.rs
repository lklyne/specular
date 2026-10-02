//! `specular-app`: the winit shell for the Rust CEF spike.
//!
//! Usage: `specular-app [path/to/file.canvas]`. Without a path it lays out a
//! demo grid of pages. Pages are hosted by CEF when built with `--features
//! cef`, otherwise by the synthetic source (non-representative CPU frames).

mod app;
mod scene;
mod source_select;

use anyhow::Context as _;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

fn main() -> anyhow::Result<()> {
    if let Some(code) = source_select::run_subprocess_if_needed() {
        std::process::exit(code);
    }
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let canvas_path = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    let pages = scene::load_pages(canvas_path.as_deref())?;
    let source = source_select::create_source()?;
    tracing::info!(backend = source.name(), pages = pages.len(), "starting");

    let event_loop = EventLoop::new().context("creating event loop")?;
    let mut app = app::App::new(source, pages);
    event_loop.run_app(&mut app).context("running event loop")?;
    app.into_result()
}
