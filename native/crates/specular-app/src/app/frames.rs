//! The runtime's loop turn and its frames: taking in what happened since
//! the last turn, saying whether a frame is owed, and drawing one.
//!
//! A shell calls [`turn`](Runtime::turn) every time its loop wakes, then
//! draws only if [`frame_wanted`](Runtime::frame_wanted) says so and
//! otherwise sleeps until [`next_turn`](Runtime::next_turn). A shell that
//! draws every turn regardless still works; it just never rests.

use std::time::{Duration, Instant};

use specular_bench::{FrameWork, PaintPolicy};
use specular_compositor::FrameSample;
use specular_doc::EntityId;

use super::demand;
use super::runtime::{Runtime, ShellWindow, unix_ms};

/// The least time between two runs of the chores. A page source's own timer
/// can wake the loop far more often than they need to run.
const CHORE_INTERVAL: Duration = Duration::from_millis(8);

impl<W: ShellWindow> Runtime<W> {
    /// One loop turn of everything that is not drawing: what the pages
    /// reported, and on a timer the clock, the files of the space, and what
    /// the worker threads finished.
    pub fn turn(&mut self) {
        let now = Instant::now();
        self.take_page_events();
        if (self.chores_at).is_none_or(|at| now.duration_since(at) >= CHORE_INTERVAL) {
            self.chores_at = Some(now);
            self.chores(now);
        }
    }

    /// Whether something a frame shows has changed since the last one was
    /// drawn, so that [`draw`](Self::draw) is worth calling now.
    pub fn frame_wanted(&mut self) -> bool {
        self.demand.wanted(Instant::now())
    }

    /// When to call [`turn`](Self::turn) again if no event comes first.
    pub fn next_turn(&self) -> Instant {
        let now = Instant::now();
        let wake = self.demand.next_turn(&self.app, now);
        if self.source.paints_on_pump() {
            // Its pages only paint when pumped, so keep to the display's pace.
            return wake.min(now + CHORE_INTERVAL);
        }
        wake
    }

    /// The user did something. Frames keep coming for a moment afterwards,
    /// so the display holds its refresh rate through a pause in a gesture.
    pub fn input(&mut self) {
        self.demand.input();
    }

    /// Pumps the page source and hands on what its pages reported.
    fn take_page_events(&mut self) {
        self.source.pump();
        let mut events = std::mem::take(&mut self.events);
        self.source.drain_events(&mut events);
        for event in events.drain(..) {
            self.handle_page_event(event);
        }
        self.events = events;
    }

    /// What is looked at on a timer and not on an event: the clock, the
    /// pages' paint rates, the files and the loader threads.
    fn chores(&mut self, now: Instant) {
        let (effects, stale) = demand::tick(&mut self.app, unix_ms());
        if stale {
            self.demand.changed();
        }
        self.run_all(effects);
        // A page's paint rate and texture scale settle a while after the
        // camera stops, with no frame to hang the change on.
        if self.options.paint_policy == PaintPolicy::ElectronLod
            && let Some(viewport) = self.gpu.as_ref().map(W::logical_viewport)
        {
            self.update_paint_lod(viewport, now);
        }
        // A page that has stopped painting pins the surfaces it last cycled
        // through until something lets them go.
        if let Some(gpu) = self.gpu.as_mut() {
            gpu.compositor_mut().release_idle(now);
        }
        self.sync_files();
        self.take_loaded_image();
        self.take_read_notes();
        self.flush_drops();
    }

    /// Renders and presents one frame of the app as it stands. `None` when
    /// the window had no frame to give.
    pub fn draw(&mut self) -> Option<FrameSample> {
        let viewport = self.gpu.as_ref()?.logical_viewport();
        if self.options.paint_policy == PaintPolicy::ElectronLod {
            self.update_paint_lod(viewport, Instant::now());
        }
        let started = Instant::now();
        let mut scene = if self.options.chrome {
            specular_scene::view(&self.app, viewport)
        } else {
            specular_scene::view_without_chrome(&self.app, viewport)
        };
        specular_scene::draw_panels(&self.app, &mut scene);
        let view = started.elapsed();
        let camera = self.app.session().camera;
        // Text keeps its raster size while the zoom moves, and the first
        // frame at a steady zoom sharpens it.
        let zooming = (camera.zoom - self.drawn_zoom).abs() > f32::EPSILON;
        self.drawn_zoom = camera.zoom;
        let hosts = &self.hosts;
        let page_of = |entity: &EntityId| hosts.get(entity).map(|host| host.page);
        let started = Instant::now();
        let rendered = (self.gpu.as_mut()?).render(camera, zooming, &mut scene, &page_of);
        let Some(stats) = rendered else {
            tracing::debug!("no frame to draw into");
            self.demand.refused(Instant::now());
            return None;
        };
        // Text held at a zoom's old glyph size, or drawn part of a pixel
        // off by a pan, is put right by one more frame.
        self.demand.drawn(zooming || stats.text.settling);
        let presented_at = Instant::now();
        self.last_work = FrameWork {
            view_ms: view.as_secs_f64() * 1_000.0,
            // The drawable, the present and the wait for vsync between them.
            gpu_ms: (presented_at - started)
                .saturating_sub(stats.times.total())
                .as_secs_f64()
                * 1_000.0,
            ..crate::headless::work_of(&stats)
        };
        let sample = FrameSample {
            presented_at,
            stats: stats.render,
            input_to_present: self.latency.presented(presented_at),
        };
        if let Some(latency) = sample.input_to_present {
            tracing::debug!(?latency, "input to present");
        }
        self.report_note_heights();
        Some(sample)
    }
}
