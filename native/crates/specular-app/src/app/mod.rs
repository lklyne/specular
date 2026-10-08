//! The winit application: one window, one compositor, one page source.
//!
//! Each loop turn: advance the bench (if any), pump the source, feed its
//! events to the compositor, render, present, then report a
//! [`FrameSample`].

mod api_run;
mod asset_run;
mod bench;
mod clipboard_run;
mod drop_run;
mod effects;
#[cfg(target_os = "macos")]
mod file_menu;
mod gpu_window;
mod image_run;
mod input;
mod lod;
#[cfg(target_os = "macos")]
mod menu_bar;
mod note_run;
mod page_events;
mod runtime;
mod settings;
mod space_run;
mod title;

use std::path::PathBuf;
use std::time::Duration;

use glam::Vec2;
use specular_bench::{GestureProfile, PaintPolicy, STEP_INTERVAL};
use specular_compositor::FrameObserver as _;
use specular_core::{Camera, PageSource};
use specular_doc::Document;
use specular_interact::Event;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::ModifiersState;
use winit::window::WindowId;

pub(crate) use self::api_run::ShellEvent;
use self::gpu_window::GpuWindow;
pub use self::runtime::{Opening, PageOf, Runtime, RuntimeOptions, ShellWindow};
use crate::bench_run::BenchRun;
use crate::space::SpaceStart;
use crate::translate::ClickCounter;

/// Camera a canvas with no saved one opens at, and each bench profile starts
/// from.
const START_CAMERA: Camera = Camera {
    pan: Vec2::new(40.0, 40.0),
    zoom: 0.25,
};

/// How the shell runs, from the command line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunOptions {
    /// The `.canvas` file the document came from, if it came from one and
    /// no space is opened: a benchmark's.
    pub(crate) canvas: Option<PathBuf>,
    /// The space folder to open once the window exists. With one, the
    /// document the shell was given is not shown.
    pub(crate) space: Option<SpaceStart>,
    /// Profiles to run then exit; `None` stays interactive.
    pub(crate) bench: Option<Vec<GestureProfile>>,
    /// Settle time before the first bench profile.
    pub(crate) warmup: Duration,
    /// Whether the source's frames may be compared with Electron's.
    pub(crate) representative_source: bool,
    /// How pages are throttled.
    pub(crate) paint_policy: PaintPolicy,
    /// Window size in logical pixels; `None` takes the platform default.
    pub(crate) window: Option<(u32, u32)>,
    /// Whether the chrome layer (borders, selection, annotations, tools) runs.
    pub(crate) chrome: bool,
    /// Page-bound annotations seeded at startup.
    pub(crate) annotations: usize,
}

/// The winit shell: turns window events into [`Event`]s for the
/// [`Runtime`], and runs benchmarks through it.
pub(crate) struct Shell {
    runtime: Runtime<GpuWindow>,
    /// What to show once the window exists.
    opening: Option<Opening>,
    /// Wakes the event loop from another thread.
    wake: EventLoopProxy<ShellEvent>,
    #[cfg(target_os = "macos")]
    menu: Option<menu_bar::MenuBar>,
    modifiers: ModifiersState,
    /// The pointer's latest position in logical window pixels.
    cursor: Option<Vec2>,
    clicks: ClickCounter,
    options: RunOptions,
    bench: Option<BenchRun>,
}

impl Shell {
    /// A shell showing `document` in `source`; with bench profiles in
    /// `options` it runs them after the warmup and exits instead of staying
    /// interactive.
    pub(crate) fn new(
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
    pub(crate) fn into_result(self) -> anyhow::Result<()> {
        self.runtime.into_result()
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let gpu = GpuWindow::new(
            event_loop,
            self.options.window,
            self.options.bench.is_some(),
        )?;
        let step_interval = gpu.refresh_interval().unwrap_or(STEP_INTERVAL);
        self.runtime.attach_window(gpu);
        // The toolbar and the popup are part of the chrome layer. A
        // benchmark measures the canvas, so it runs without them.
        let panels = self.options.chrome && self.options.bench.is_none();
        self.runtime.dispatch(Event::BuiltinPanels(panels));
        if let Some(opening) = self.opening.take() {
            self.runtime.open(opening)?;
        }
        let Some(profiles) = self.options.bench.take() else {
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
            self.start_bench(profiles, step_interval);
        }
        Ok(())
    }

    fn redraw(&mut self) -> anyhow::Result<()> {
        let Some(viewport) = self.runtime.gpu.as_ref().map(GpuWindow::logical_viewport) else {
            return Ok(());
        };
        if self.tick_bench(viewport)? {
            return Ok(());
        }
        let Some(sample) = self.runtime.draw() else {
            return Ok(());
        };
        if let Some(bench) = self.bench.as_mut() {
            bench.on_frame(&sample);
        }
        Ok(())
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
        event_loop.set_control_flow(ControlFlow::Poll);
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
            WindowEvent::DroppedFile(path) => self.runtime.drop_files([path], self.cursor),
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw() {
                    self.runtime.fail(error);
                }
            }
            other => self.on_input(other),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.runtime.closing {
            if self.runtime.poll_shutdown() {
                event_loop.exit();
            }
            return;
        }
        self.runtime.turn();
        #[cfg(target_os = "macos")]
        self.run_menu();
        // A benchmark window keeps the title it opened with.
        if self.bench.is_none() {
            self.runtime.refresh_title();
        }
        if let Some(gpu) = self.runtime.gpu.as_ref() {
            gpu.window.request_redraw();
        }
    }
}
