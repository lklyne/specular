//! The winit window: the `specular-app` binary's shell. It turns window
//! events into [`Event`]s for the [`Runtime`]. The rest of it is
//! `input.rs`, `turn.rs`, `file_menu.rs`, `gpu_window.rs` and `menu_bar/`
//! beside this file, and `translate.rs`. Nothing else names winit.

use std::path::PathBuf;

use anyhow::Context as _;
use glam::Vec2;
use specular_bench::PaintPolicy;
use specular_core::PageSource;
use specular_doc::Document;
use specular_interact::Event;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

use super::gpu_window::GpuWindow;
#[cfg(target_os = "macos")]
use super::menu_bar;
use super::{Opening, Runtime, RuntimeOptions, ShellEvent};
use crate::bench_drive::{Bench, BenchOptions};
use crate::launch::Launch;
use crate::source_select::{self, Host};
use crate::space::SpaceStart;
use crate::translate::ClickCounter;

/// How the shell runs, from the command line.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RunOptions {
    /// The `.canvas` file the document came from, if it came from one and
    /// no space is opened: a benchmark's.
    canvas: Option<PathBuf>,
    /// The space folder to open once the window exists. With one, the
    /// document the shell was given is not shown.
    space: Option<SpaceStart>,
    /// The benchmark to run then exit; `None` stays interactive.
    bench: Option<BenchOptions>,
    /// How pages are throttled.
    paint_policy: PaintPolicy,
    /// Window size in logical pixels; `None` takes the platform default.
    window: Option<(u32, u32)>,
    /// Whether the chrome layer (borders, selection, annotations, tools) runs.
    chrome: bool,
}

/// The winit shell: turns window events into [`Event`]s for the
/// [`Runtime`], and runs benchmarks through it.
pub(crate) struct Shell {
    pub(super) runtime: Runtime<GpuWindow>,
    /// What to show once the window exists.
    pub(super) opening: Option<Opening>,
    /// Wakes the event loop from another thread.
    pub(super) wake: EventLoopProxy<ShellEvent>,
    #[cfg(target_os = "macos")]
    pub(super) menu: Option<menu_bar::MenuBar>,
    pub(super) modifiers: ModifiersState,
    /// The pointer's latest position in logical window pixels.
    pub(super) cursor: Option<Vec2>,
    pub(super) clicks: ClickCounter,
    pub(super) options: RunOptions,
    /// The benchmark, while one runs.
    pub(super) bench: Option<Bench>,
}

impl Shell {
    /// A shell showing `document` in `source`; with bench profiles in
    /// `options` it runs them after the warmup and exits instead of staying
    /// interactive.
    fn new(
        source: Box<dyn PageSource>,
        document: Document,
        mut options: RunOptions,
        wake: EventLoopProxy<ShellEvent>,
    ) -> Self {
        let runtime = Runtime::new(
            source,
            RuntimeOptions {
                canvas: options.canvas.clone(),
                paint_policy: options.paint_policy,
                chrome: options.chrome,
                // A benchmark neither reads nor writes settings.
                settings: options.bench.is_none(),
            },
        );
        Self {
            runtime,
            opening: Some(Opening {
                space: options.space.take(),
                ask: None,
                document: Some(document),
            }),
            wake,
            #[cfg(target_os = "macos")]
            menu: None,
            modifiers: ModifiersState::empty(),
            cursor: None,
            clicks: ClickCounter::default(),
            options,
            bench: None,
        }
    }

    /// The first fatal error hit inside the event loop, if any.
    fn into_result(self) -> anyhow::Result<()> {
        self.runtime.into_result()
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let gpu = GpuWindow::new(
            event_loop,
            self.options.window,
            self.options.bench.is_some(),
        )?;
        let refresh = gpu.refresh_interval();
        let system = gpu.window.theme().map(crate::translate::appearance);
        self.runtime.attach_window(gpu);
        if let Some(system) = system {
            self.runtime.dispatch(Event::SystemAppearance(system));
        }
        // The toolbar and the popup are part of the chrome layer. A
        // benchmark measures the canvas, so it runs without them.
        let panels = self.options.chrome && self.options.bench.is_none();
        self.runtime.dispatch(Event::BuiltinPanels(panels));
        if let Some(opening) = self.opening.take() {
            self.runtime.open(opening)?;
        }
        let Some(bench) = self.options.bench.take() else {
            // A benchmark keeps winit's default menu: fewer moving parts in
            // a measured run. It takes no outside input either, so it has
            // no API.
            #[cfg(target_os = "macos")]
            self.install_menu();
            let wake = self.wake.clone();
            self.runtime.start_api(move || {
                // The loop is gone when the app is quitting.
                let _ = wake.send_event(ShellEvent::Api);
            });
            return Ok(());
        };
        if !self.runtime.closing {
            let bench = Bench::start(&mut self.runtime, bench, "winit", refresh);
            self.bench = Some(bench);
        }
        Ok(())
    }

    /// Draws and presents one frame of the app as it stands.
    fn redraw(&mut self) {
        let Some(sample) = self.runtime.draw() else {
            return;
        };
        if let Some(bench) = self.bench.as_mut() {
            bench.presented(&self.runtime, &sample);
        }
    }
}

impl ApplicationHandler<ShellEvent> for Shell {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: ShellEvent) {
        match event {
            ShellEvent::Api => self.runtime.serve_api(),
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.runtime.gpu.is_some() {
            return;
        }
        if let Err(error) = self.init(event_loop) {
            self.runtime.fail(error);
        }
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => self.runtime.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.runtime.gpu.as_mut() {
                    gpu.resize(size);
                    let viewport = gpu.logical_viewport();
                    self.runtime.dispatch(Event::ViewportResized(viewport));
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.runtime.on_scale_factor_changed(scale_factor);
            }
            WindowEvent::ThemeChanged(theme) => {
                let system = crate::translate::appearance(theme);
                self.runtime.dispatch(Event::SystemAppearance(system));
            }
            WindowEvent::DroppedFile(path) => self.runtime.drop_files([path], self.cursor),
            // The system asks too, after uncovering the window.
            WindowEvent::RedrawRequested => self.redraw(),
            other => self.on_input(other),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.runtime.closing {
            // Shutting the source down takes turns of the loop.
            event_loop.set_control_flow(ControlFlow::Poll);
            if self.runtime.poll_shutdown() {
                event_loop.exit();
            }
            return;
        }
        if self.runtime.take_space_dialog().is_some() {
            self.choose_space();
        }
        self.turn(event_loop);
    }
}

/// Opens the winit window on `launch` and runs until it closes.
pub fn run_window(launch: Launch) -> anyhow::Result<()> {
    let bench = launch.bench();
    let Launch {
        run,
        space,
        document,
        ..
    } = launch;
    // winit must create the macOS application object before CEF initializes,
    // or CEF installs its own and winit panics.
    let mut event_loop = EventLoop::<ShellEvent>::with_user_event();
    // The shell installs its own menu bar, except in a benchmark.
    #[cfg(target_os = "macos")]
    winit::platform::macos::EventLoopBuilderExtMacOS::with_default_menu(
        &mut event_loop,
        run.bench.is_some(),
    );
    let event_loop = event_loop.build().context("creating event loop")?;
    let source = source_select::create_source(run.source, Host::Window, None)?;
    tracing::info!(
        backend = source.name(),
        entities = document.entities().count(),
        paint_policy = run.paint_policy.name(),
        "starting"
    );

    let options = RunOptions {
        canvas: run.canvas.filter(|_| space.is_none()),
        space,
        bench,
        paint_policy: run.paint_policy,
        window: run.window,
        chrome: run.chrome,
    };
    let mut app = Shell::new(source, document, options, event_loop.create_proxy());
    event_loop.run_app(&mut app).context("running event loop")?;
    app.into_result()
}
