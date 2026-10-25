//! `--bench` in a window, for whichever shell opened it.
//!
//! A shell's loop gives the run three things: a [`step`](Bench::step) each
//! turn after [`Runtime::turn`], the frame it then presented
//! ([`presented`](Bench::presented)), and the time it spent on a frame
//! outside the runtime ([`worked`](Bench::worked)). The run moves the
//! camera through the runtime as a wheel event would, prints one JSON line
//! a profile when the last has run, and starts the exit.
//!
//! A profile is stepped once a presented frame, however the shell's loop is
//! woken: by a redraw that blocks on the display, or by a display link.

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_bench::{FrameWork, GestureProfile, STEP_INTERVAL};
use specular_compositor::{FrameObserver as _, FrameSample};
use specular_core::Camera;
use specular_doc::ItemId;
use specular_interact::{Action, Event};

use crate::app::{Runtime, ShellWindow};
use crate::bench_run::{BenchRun, BenchTick, RunSource};
use crate::headless::START_CAMERA;

/// What a window benchmark runs, from the command line.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchOptions {
    /// The profiles, in run order.
    pub(crate) profiles: Vec<GestureProfile>,
    /// Settle time before the first.
    pub(crate) warmup: Duration,
    /// Whether the source's frames may be compared with Electron's.
    pub(crate) representative: bool,
    /// Page-bound annotations seeded at startup.
    pub(crate) annotations: usize,
}

/// What one loop turn of the run asks of the shell.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Turn {
    /// The camera to move to: one step of a gesture.
    camera: Option<Camera>,
    /// Present a frame although nothing changed.
    present: bool,
    /// Whether a step ran, moving or not.
    stepped: bool,
    /// Every profile has run.
    finished: bool,
    /// When to come back if no frame is presented first.
    next: Option<Instant>,
}

/// A benchmark running in a shell's window.
#[derive(Debug)]
pub struct Bench {
    run: BenchRun,
    /// When the run last stepped.
    stepped_at: Option<Instant>,
    /// Whether a frame has been presented since.
    presented: bool,
    /// Whether the run is in a wait already counted as a skipped turn.
    waiting: bool,
    /// What the next presented frame's `update` took.
    update: Duration,
    finished: bool,
}

impl Bench {
    /// Starts `options` on what `runtime` shows, once the warmup has passed.
    /// `shell` names the window for the report, and `refresh` is its
    /// display's refresh interval, when the platform says.
    pub fn start<W: ShellWindow>(
        runtime: &mut Runtime<W>,
        options: BenchOptions,
        shell: &'static str,
        refresh: Option<Duration>,
    ) -> Self {
        if runtime.options.chrome {
            // The selection outline and handles belong in every measured frame.
            let first = runtime.app.pages().next().map(|(id, ..)| id.clone());
            let selection = first.into_iter().map(ItemId::Entity).collect();
            runtime.dispatch(Event::Action(Action::Select(selection)));
        }
        let source = RunSource {
            name: runtime.source.name(),
            representative: options.representative,
            pages: runtime.hosts.len(),
            paint_policy: runtime.options.paint_policy,
            chrome: runtime.options.chrome,
            annotations: options.annotations,
            canvas: (runtime.options.canvas.as_deref())
                .and_then(std::path::Path::file_name)
                .map(|name| name.to_string_lossy().into_owned()),
            shell,
        };
        let run = BenchRun::new(
            options.profiles,
            options.warmup,
            refresh.unwrap_or(STEP_INTERVAL),
            START_CAMERA,
            source,
            Instant::now(),
        );
        Self::of(run)
    }

    fn of(run: BenchRun) -> Self {
        Self {
            run,
            stepped_at: None,
            presented: false,
            waiting: false,
            update: Duration::ZERO,
            finished: false,
        }
    }

    /// One loop turn of the run, after [`Runtime::turn`] and before the
    /// shell asks whether a frame is wanted. Returns when to turn again if
    /// no frame is presented first. Once the last profile has run this
    /// prints the report and starts the exit.
    pub fn step<W: ShellWindow>(
        &mut self,
        runtime: &mut Runtime<W>,
        now: Instant,
    ) -> Option<Instant> {
        if self.finished {
            return None;
        }
        let viewport = runtime.gpu.as_ref().map(W::logical_viewport)?;
        if self.stepped_at.is_none() && self.run.waits_until().is_none_or(|until| now >= until) {
            // Two runs compare only if they drew the same viewport.
            let (width, height) = (viewport.x, viewport.y);
            let scale = runtime.gpu.as_ref().map(W::scale_factor);
            tracing::info!(width, height, scale, "the first profile starts");
        }
        let started = Instant::now();
        let turn = self.pace(now, runtime.app.session().camera, viewport);
        if turn.finished {
            self.finish(runtime);
            return None;
        }
        if turn.present {
            runtime.demand.changed();
        }
        if let Some(camera) = turn.camera {
            // A step of a gesture, as a wheel event is.
            runtime.input();
            runtime.dispatch(Event::Action(Action::SetCamera(camera)));
        }
        if turn.stepped {
            self.update += started.elapsed();
        }
        turn.next
    }

    /// Steps the run: once a presented frame, or once a refresh when the
    /// last step changed nothing and so drew nothing. While the run only
    /// waits (the warmup, a gap, the idle profile) it is left alone until
    /// the wait ends.
    fn pace(&mut self, now: Instant, camera: Camera, viewport: Vec2) -> Turn {
        if let Some(until) = self.run.waits_until().filter(|&until| now < until) {
            let next = Some(until);
            if self.run.keeps_presenting(now) {
                return Turn {
                    present: true,
                    next,
                    ..Turn::default()
                };
            }
            if !self.waiting {
                // One turn with nothing to draw, however long the wait.
                self.run.on_skipped();
            }
            self.waiting = true;
            self.presented = false;
            return Turn {
                next,
                ..Turn::default()
            };
        }
        self.waiting = false;
        let interval = self.run.step_interval();
        let due = (self.stepped_at).is_none_or(|at| now.duration_since(at) >= interval);
        if !self.presented && !due {
            return Turn {
                next: self.stepped_at.map(|at| at + interval),
                ..Turn::default()
            };
        }
        if !self.presented {
            self.run.on_skipped();
        }
        self.presented = false;
        self.stepped_at = Some(now);
        let mut moved = camera;
        let finished = self.run.tick(now, &mut moved, viewport / 2.0) == BenchTick::Finished;
        Turn {
            camera: (moved != camera).then_some(moved),
            present: false,
            stepped: true,
            finished,
            next: Some(now + interval),
        }
    }

    /// The shell presented `sample`, the frame [`Runtime::draw`] returned.
    pub fn presented<W: ShellWindow>(&mut self, runtime: &Runtime<W>, sample: &FrameSample) {
        self.record(sample, runtime.last_work);
    }

    fn record(&mut self, sample: &FrameSample, work: FrameWork) {
        self.run.on_frame(sample);
        self.run.on_work(FrameWork {
            update_ms: std::mem::take(&mut self.update).as_secs_f64() * 1_000.0 + work.update_ms,
            ..work
        });
        self.presented = true;
    }

    /// Time the shell spent on a frame that the runtime did not: bringing
    /// its own toolkit in step with the app. It is counted in the next
    /// presented frame's `update`.
    pub fn worked(&mut self, time: Duration) {
        self.update += time;
    }

    fn finish<W: ShellWindow>(&mut self, runtime: &mut Runtime<W>) {
        self.finished = true;
        for report in self.run.reports() {
            match serde_json::to_string(report) {
                Ok(line) => println!("{line}"),
                Err(error) => {
                    runtime.fail(anyhow::Error::new(error).context("printing a bench line"));
                    return;
                }
            }
        }
        runtime.exit();
    }
}
