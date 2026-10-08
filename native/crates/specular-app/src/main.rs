//! `specular-app`: the winit shell for the Rust CEF spike.
//!
//! Hosts pages from a `.canvas` file (or a demo grid) on a pannable, zoomable
//! canvas, in CEF (`--source cef`, needs `--features cef`) or the synthetic
//! source (non-representative CPU frames). `--bench` replays the Electron
//! pan/zoom profiles and prints frame timing. `--snapshot` and `--script`
//! draw into PNG files with no window. `--help` prints the usage.

fn main() -> anyhow::Result<()> {
    if let Some(code) = specular_app::run_subprocess_if_needed() {
        std::process::exit(code);
    }
    match specular_app::launch(std::env::args_os().skip(1))? {
        Some(launch) => specular_app::run_window(launch),
        None => Ok(()),
    }
}
