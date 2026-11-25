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
//! - [`input`] and [`keys`]: the canvas's pointer, wheel, pinch, keys and
//!   input method, as the app's events.
//! - [`panel`]: the same toolbar as a Luau script (step S7, ADR 0055),
//!   chosen with `--chrome luau`.
//! - [`pages`]: the page source, synthetic or CEF; [`cef`] (feature `cef`)
//!   starts CEF, wakes Rune when a page paints, and waits headless runs out
//!   (step S5).

pub mod canvas;
#[cfg(feature = "cef")]
pub mod cef;
pub mod chrome;
pub mod files;
pub mod input;
pub mod keys;
pub mod pages;
pub mod panel;
pub mod shell;

use std::path::PathBuf;

use rune::{App, AppSpec, Waker};
use specular_core::Camera;

use crate::pages::{PumpKind, SourceKind};
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
    /// Samples per pixel of the scene pass: 4, or 1 for a backend with no
    /// multisampling.
    pub samples: u32,
    /// What hosts the pages.
    pub source: SourceKind,
    /// Who turns CEF's loop, with `--source cef`.
    pub pump: PumpKind,
    /// A headless run (`--snap`, `--tree`, `--script`): no window's run
    /// loop turns.
    pub headless: bool,
    /// `--probe-keys SECONDS`: digit 1 and keypad 1 at that time.
    pub probe_keys_at: Option<f64>,
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
            samples: 4,
            source: SourceKind::Synthetic,
            pump: PumpKind::Timer,
            headless: false,
            probe_keys_at: None,
        }
    }
}

/// `AppSpec::setup`: a watched script reload asks for a frame through this.
/// With CEF in a window, a page's paint asks for a frame through it too.
fn setup(shell: &mut Shell, waker: Waker) {
    #[cfg(feature = "cef")]
    if shell.pages.name == "cef" && shell.pages.pump == PumpKind::Timer {
        cef::watch_paints(&shell.pages, waker.clone());
    }
    if let Some(panel) = &shell.panel {
        panel.set_waker(waker);
    }
}

/// Starts the page source `options` asks for.
#[cfg_attr(
    not(feature = "cef"),
    expect(unused_variables, reason = "only CEF replaces the pages")
)]
fn start_pages(shell: &mut Shell, options: &Options) {
    match options.source {
        SourceKind::Synthetic => {}
        #[cfg(feature = "cef")]
        SourceKind::Cef => match cef::start(options.pump, options.headless) {
            Ok(pages) => shell.pages = pages,
            Err(error) => {
                eprintln!("rune-shell: {error}");
                std::process::exit(1);
            }
        },
        #[cfg(not(feature = "cef"))]
        SourceKind::Cef => {
            eprintln!("rune-shell: this build has no CEF; rebuild with --features cef");
            std::process::exit(2);
        }
    }
}

/// A close asked for while CEF runs waits for it to stop: closing its
/// browsers and `cef::shutdown` both need the run loop still turning, and
/// `poll` turns the shutdown until it is done, then closes the window.
#[cfg(feature = "cef")]
fn on_close_request(shell: &mut Shell, tx: &mut rune::Tx) -> rune::CloseResponse {
    if shell.pages.name != "cef" || shell.closing.is_some() {
        return rune::CloseResponse::Close;
    }
    eprintln!("rune-shell: close asked for; shutting the page source down");
    shell.closing = Some(std::time::Instant::now());
    tx.wake_after(std::time::Duration::ZERO);
    rune::CloseResponse::Defer
}

/// A quit (Command-Q) does not ask first: the source is stopped at once,
/// inside the handler, which is what the run-loop timer exists to avoid.
#[cfg(feature = "cef")]
fn on_quit(shell: &mut Shell, _: &mut rune::Tx) {
    if shell.pages.name == "cef" && !shell.closed {
        shell.pages.get().shutdown();
    }
}

/// The Rune app for `options`, before `run` or `headless_runtime`.
pub fn app(options: Options) -> App<Shell> {
    let spec = AppSpec {
        poll: Some(shell::poll),
        setup: Some(setup),
        on_key: Some(keys::on_key),
        ..AppSpec::new(
            "Specular (Rune spike)",
            options.size,
            Shell::empty,
            chrome::view,
        )
    };
    // Modifier keys alone reach `on_key`, as the Kit sends flag changes.
    let app = App::new(spec);
    #[cfg(feature = "cef")]
    let app = app.on_close_request(on_close_request).on_quit(on_quit);
    app.modifier_keys().on_ime(keys::on_ime).init(move || {
        let clock = if options.still {
            Clock::Still
        } else {
            Clock::Running
        };
        let mut shell = Shell::new(options.size, options.scale, clock);
        start_pages(&mut shell, &options);
        shell.probe_keys_at = options.probe_keys_at;
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
        #[cfg(feature = "cef")]
        if options.source == SourceKind::Cef && options.headless {
            cef::settle(&mut shell);
        }
        shell
    })
}
