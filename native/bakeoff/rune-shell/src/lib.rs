//! The Rune spike (docs/plans/rune-spike.md, ADR 0051): Specular's
//! compositor drawing inside a Rune frame, under a toolbar Rune draws from
//! `toolbar(&App)`.
//!
//! Throwaway code. Three parts:
//!
//! - [`shell`]: the Rune app's state. An [`App`](specular_interact::App),
//!   the synthetic pages and a cut-down effect runner. No GPU.
//! - [`chrome`]: the Rune view, a toolbar built from the panel model.
//! - [`canvas`]: the Rune `Compositor` that hosts `specular-compositor` on
//!   Rune's device and draws Rune's UI over it.
//! - [`panel`]: the same toolbar as a Luau script (step S7, ADR 0055),
//!   chosen with `--chrome luau`.

pub mod canvas;
pub mod chrome;
pub mod files;
pub mod panel;
pub mod shell;

use std::path::PathBuf;

use rune::{App, AppSpec, Waker};
use specular_core::Camera;

use crate::panel::Panel;
use crate::shell::{Clock, Shell};

/// The window's size in points when the command line gives none.
pub const WINDOW: (f32, f32) = (1200.0, 800.0);

/// Which chrome draws the toolbar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Chrome {
    /// `chrome.rs`: Rune's builders called from Rust.
    #[default]
    Rust,
    /// `panels/toolbar/init.luau`, or the same script in another folder. A
    /// save of a file in the folder reloads the panel.
    Luau(PathBuf),
}

/// The folder the shipped toolbar script is in.
pub fn toolbar_panel() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("panels/toolbar")
}

/// What the spike's own arguments ask for. Everything else on the command
/// line is Rune's (`--snap`, `--tree`, `--script`, `--size`, `--scale` ...).
#[derive(Debug, Clone)]
pub struct Options {
    /// The `.canvas` file to show.
    pub canvas: Option<PathBuf>,
    /// The camera to open at. `None` fits the document.
    pub camera: Option<Camera>,
    /// Hold the app's clock still, as `specular --snapshot` does, so a gif
    /// shows the frame the reference shows.
    pub still: bool,
    /// The viewport in points. A headless run is told Rune's `--size` too.
    pub size: (f32, f32),
    /// What draws the toolbar.
    pub chrome: Chrome,
    /// The device pixels per point pages are first hosted at: Rune's
    /// `--scale` in a headless run. A window finds its own in its first
    /// frame.
    pub scale: f32,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            canvas: None,
            camera: None,
            still: false,
            size: WINDOW,
            chrome: Chrome::Rust,
            scale: 1.0,
        }
    }
}

/// `AppSpec::setup`: a watched script reload asks for a frame through this.
fn setup(shell: &mut Shell, waker: Waker) {
    if let Some(panel) = &shell.panel {
        panel.set_waker(waker);
    }
}

/// The Rune app for `options`, before `run` or `headless_runtime`.
pub fn app(options: Options) -> App<Shell> {
    let spec = AppSpec {
        poll: Some(shell::poll),
        setup: Some(setup),
        ..AppSpec::new(
            "Specular (Rune spike)",
            options.size,
            Shell::empty,
            chrome::view,
        )
    };
    App::new(spec).init(move || {
        let clock = if options.still {
            Clock::Still
        } else {
            Clock::Running
        };
        let mut shell = Shell::new(options.size, options.scale, clock);
        if let Chrome::Luau(dir) = &options.chrome {
            match Panel::from_dir(dir) {
                Ok(panel) => {
                    panel.watch(dir);
                    shell.panel = Some(panel);
                }
                Err(error) => {
                    eprintln!("rune-shell: reading {}: {error}", dir.display());
                    std::process::exit(1);
                }
            }
        }
        if let Some(canvas) = &options.canvas
            && let Err(error) = shell.open(canvas, options.camera)
        {
            eprintln!("rune-shell: {error}");
            std::process::exit(1);
        }
        shell
    })
}
