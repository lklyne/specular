//! `--bench`: drives the camera through gesture profiles and reports
//! presented-frame timing, mirroring Electron's `POST /perf/pan-zoom/run`.
//!
//! One profile step is applied per rendered frame (the Electron test steps
//! once per display refresh too), from the same start camera each time, and
//! zooms anchor at the viewport centre.

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_bench::{
    FrameWork, GestureProfile, GestureStep, PHASE_GAP, PaintPolicy, PhaseRecorder, PresentedFrame,
    ProfileId, ProfileLine, WorkRecorder, build_steps, process_cpu_time,
};
use specular_compositor::{FrameObserver, FrameSample};
use specular_core::Camera;

/// How long before the first profile frames are presented again.
const LEAD_IN: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Warmup {
        until: Instant,
    },
    Running {
        profile: usize,
        step: usize,
    },
    /// The idle profile: nothing is stepped, so the run only waits.
    Idle {
        profile: usize,
        until: Instant,
    },
    Gap {
        until: Instant,
        next: usize,
    },
    Done,
}

#[derive(Debug)]
struct Recording {
    frames: PhaseRecorder,
    work: WorkRecorder,
    max_paint_to_submit: Option<Duration>,
    max_shapes_drawn: u32,
    /// When the profile began, and the process's CPU time then.
    began: Instant,
    cpu_began: Option<Duration>,
}

/// What the app should do after [`BenchRun::tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BenchTick {
    /// Keep rendering.
    Continue,
    /// Every profile has run; exit.
    Finished,
}

/// The page source a run measures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunSource {
    /// Backend name for the report (`cef`, `synthetic`).
    pub(crate) name: &'static str,
    /// Whether the backend's frames may be compared with Electron's.
    pub(crate) representative: bool,
    /// Pages on the canvas.
    pub(crate) pages: usize,
    /// How pages were throttled.
    pub(crate) paint_policy: PaintPolicy,
    /// Whether the chrome layer was drawn every frame.
    pub(crate) chrome: bool,
    /// Annotations drawn every frame.
    pub(crate) annotations: usize,
    /// The canvas shown, by file name.
    pub(crate) canvas: Option<String>,
    /// The shell whose window presents the frames (`winit`, `kit`).
    pub(crate) shell: &'static str,
}

/// The benchmark state machine.
#[derive(Debug)]
pub(crate) struct BenchRun {
    profiles: Vec<GestureProfile>,
    step_interval: Duration,
    start_camera: Camera,
    source: RunSource,
    phase: Phase,
    steps: Vec<GestureStep>,
    recording: Option<Recording>,
    reports: Vec<ProfileLine>,
}

impl BenchRun {
    /// A run of `profiles` after `warmup`, stepping every `step_interval`
    /// (the display refresh interval), starting each profile from
    /// `start_camera`.
    pub(crate) fn new(
        profiles: Vec<GestureProfile>,
        warmup: Duration,
        step_interval: Duration,
        start_camera: Camera,
        source: RunSource,
        now: Instant,
    ) -> Self {
        Self {
            profiles,
            step_interval,
            start_camera,
            source,
            phase: Phase::Warmup {
                until: now + warmup,
            },
            steps: Vec::new(),
            recording: None,
            reports: Vec::new(),
        }
    }

    /// Advances the run before rendering a frame, moving `camera` when a
    /// profile is running. `anchor` is the viewport centre.
    pub(crate) fn tick(&mut self, now: Instant, camera: &mut Camera, anchor: Vec2) -> BenchTick {
        match self.phase {
            Phase::Warmup { until }
            | Phase::Gap { until, next: _ }
            | Phase::Idle { until, profile: _ }
                if now < until => {}
            Phase::Warmup { .. } => self.start(0, camera, now),
            Phase::Gap { next, .. } => self.start(next, camera, now),
            Phase::Idle { profile, .. } => self.finish(profile, now),
            Phase::Running { profile, step } => {
                if let Some(next) = self.steps.get(step) {
                    camera.apply_input_delta(next.to_input(Some(anchor)));
                    self.phase = Phase::Running {
                        profile,
                        step: step + 1,
                    };
                } else {
                    self.finish(profile, now);
                }
            }
            Phase::Done => return BenchTick::Finished,
        }
        if self.phase == Phase::Done {
            BenchTick::Finished
        } else {
            BenchTick::Continue
        }
    }

    /// Records where a drawn frame's time went.
    pub(crate) fn on_work(&mut self, work: FrameWork) {
        if let Some(recording) = self.recording.as_mut() {
            recording.work.drawn(work);
        }
    }

    /// The interval a profile is stepped at.
    pub(crate) fn step_interval(&self) -> Duration {
        self.step_interval
    }

    /// Records a loop turn that drew nothing because nothing had changed.
    pub(crate) fn on_skipped(&mut self) {
        if let Some(recording) = self.recording.as_mut() {
            recording.frames.rested();
            recording.work.skipped();
        }
    }

    /// Completed profile reports, in run order.
    pub(crate) fn reports(&self) -> &[ProfileLine] {
        &self.reports
    }

    /// When the run next has something to do with no frame presented:
    /// the end of a wait, or `None` while a gesture is stepped a frame.
    pub(crate) fn waits_until(&self) -> Option<Instant> {
        match self.phase {
            Phase::Warmup { until } | Phase::Gap { until, .. } | Phase::Idle { until, .. } => {
                Some(until)
            }
            Phase::Running { .. } | Phase::Done => None,
        }
    }

    /// Whether to keep presenting although nothing changes: through each
    /// gap and the end of the warmup. A display that is shown no frames
    /// lowers its refresh rate and takes several frames to raise it again,
    /// which a profile would record as long frames of its own.
    pub(crate) fn keeps_presenting(&self, now: Instant) -> bool {
        match self.phase {
            Phase::Gap { .. } => true,
            Phase::Warmup { until } => now + LEAD_IN >= until,
            Phase::Running { .. } | Phase::Idle { .. } | Phase::Done => false,
        }
    }

    fn start(&mut self, index: usize, camera: &mut Camera, now: Instant) {
        let Some(profile) = self.profiles.get(index) else {
            self.phase = Phase::Done;
            return;
        };
        *camera = self.start_camera;
        self.steps = build_steps(profile, self.step_interval);
        self.recording = Some(Recording {
            frames: PhaseRecorder::new(profile.id),
            work: WorkRecorder::new(),
            max_paint_to_submit: None,
            max_shapes_drawn: 0,
            began: now,
            cpu_began: process_cpu_time(),
        });
        self.phase = if profile.id == ProfileId::Idle {
            Phase::Idle {
                profile: index,
                until: now + profile.duration,
            }
        } else {
            Phase::Running {
                profile: index,
                step: 0,
            }
        };
    }

    fn finish(&mut self, index: usize, now: Instant) {
        if let (Some(profile), Some(recording)) = (self.profiles.get(index), self.recording.take())
        {
            let representative = self.source.representative && !recording.frames.saw_cpu_texture();
            let mut work = recording.work.finish();
            let wall = now.saturating_duration_since(recording.began).as_secs_f64();
            work.process_cpu = (recording.cpu_began.zip(process_cpu_time()))
                .filter(|_| wall > 0.0)
                .map(|(began, ended)| ended.saturating_sub(began).as_secs_f64() / wall);
            let mut phase = recording.frames.finish(self.step_interval);
            phase.max_shapes_drawn = Some(u64::from(recording.max_shapes_drawn));
            self.reports.push(ProfileLine {
                phase,
                label: profile.label.to_owned(),
                source: self.source.name.to_owned(),
                pages: self.source.pages,
                representative,
                step_interval_ms: millis(self.step_interval),
                max_paint_to_submit_ms: recording.max_paint_to_submit.map(millis),
                paint_policy: self.source.paint_policy,
                chrome: self.source.chrome,
                annotations: self.source.annotations,
                target: Some("window".to_owned()),
                shell: Some(self.source.shell.to_owned()),
                canvas: self.source.canvas.clone(),
                work: Some(work),
            });
        }
        self.phase = Phase::Gap {
            until: now + PHASE_GAP,
            next: index + 1,
        };
    }
}

impl FrameObserver for BenchRun {
    fn on_frame(&mut self, sample: &FrameSample) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        let stats = &sample.stats;
        recording.frames.presented(
            sample.presented_at,
            PresentedFrame {
                pages_without_texture: stats.pages_without_texture,
                cpu_textures: stats.cpu_textures,
                frames_received: stats.frames_received,
                popup_frames: stats.popup_frames,
                frames_dropped_for_pool_pressure: stats.frames_dropped_for_pool_pressure,
                outstanding_textures: stats.outstanding_textures,
                max_outstanding_textures: stats.max_outstanding_textures,
            },
        );
        recording.max_shapes_drawn = recording.max_shapes_drawn.max(stats.shapes_drawn);
        recording.max_paint_to_submit = max_option(
            recording.max_paint_to_submit,
            sample.stats.max_paint_to_submit,
        );
    }
}

fn max_option(current: Option<Duration>, candidate: Option<Duration>) -> Option<Duration> {
    match (current, candidate) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

#[cfg(test)]
mod tests {
    use specular_bench::ProfileId;
    use specular_compositor::RenderStats;

    use super::*;

    const STEP: Duration = Duration::from_millis(16);

    fn pan_profile() -> GestureProfile {
        GestureProfile {
            id: ProfileId::SlowPan,
            label: "test pan",
            duration: Duration::from_millis(48),
            pan: Vec2::new(30.0, 0.0),
            zoom_delta_y: 0.0,
        }
    }

    /// Ticks and presents one frame per `STEP` until the run finishes,
    /// returning the final camera.
    fn run_to_completion(run: &mut BenchRun, start: Instant, cpu_textures: u32) -> Camera {
        let mut camera = Camera::default();
        let mut now = start;
        for _ in 0..10_000 {
            if run.tick(now, &mut camera, Vec2::ZERO) == BenchTick::Finished {
                return camera;
            }
            run.on_frame(&FrameSample {
                presented_at: now,
                stats: RenderStats {
                    cpu_textures,
                    ..RenderStats::default()
                },
                input_to_present: None,
            });
            now += STEP;
        }
        panic!("bench never finished");
    }

    fn new_run(profiles: Vec<GestureProfile>, representative: bool, start: Instant) -> BenchRun {
        BenchRun::new(
            profiles,
            Duration::from_secs(2),
            STEP,
            Camera::default(),
            RunSource {
                name: if representative { "cef" } else { "synthetic" },
                representative,
                pages: 4,
                paint_policy: PaintPolicy::ElectronLod,
                chrome: true,
                annotations: 3,
                canvas: None,
                shell: "test",
            },
            start,
        )
    }

    #[test]
    fn a_profile_applies_its_total_pan_and_reports_its_frame_intervals() {
        // Three steps -> four frames presented while running -> three intervals.
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], true, start);
        let camera = run_to_completion(&mut run, start, 0);
        assert!((camera.pan.x - 30.0).abs() < 1e-4);
        assert_eq!(run.reports()[0].phase.frames.frames, 3);
        let json = serde_json::to_value(&run.reports()[0]).unwrap();
        assert_eq!(
            (&json["phase"], &json["draws"]),
            (&"slow-pan".into(), &3.into())
        );
    }

    #[test]
    fn cpu_textures_or_a_synthetic_source_make_a_report_non_representative() {
        for (representative, cpu_textures) in [(true, 1), (false, 0)] {
            let start = Instant::now();
            let mut run = new_run(vec![pan_profile()], representative, start);
            run_to_completion(&mut run, start, cpu_textures);
            assert!(!run.reports()[0].representative);
        }
    }

    #[test]
    fn camera_holds_still_during_warmup() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], true, start);
        let mut camera = Camera::new(Vec2::new(5.0, 5.0), 1.0);
        run.tick(start + Duration::from_millis(100), &mut camera, Vec2::ZERO);
        assert_eq!(camera.pan, Vec2::new(5.0, 5.0));
    }

    #[test]
    fn report_carries_chrome_load_and_the_largest_shape_count() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], true, start);
        let mut camera = Camera::default();
        let mut now = start;
        while run.tick(now, &mut camera, Vec2::ZERO) != BenchTick::Finished {
            run.on_frame(&FrameSample {
                presented_at: now,
                stats: RenderStats {
                    shapes_drawn: if camera.pan.x > 10.0 { 30 } else { 12 },
                    ..RenderStats::default()
                },
                input_to_present: None,
            });
            now += STEP;
        }
        let line = &run.reports()[0];
        assert_eq!(
            (line.chrome, line.annotations, line.phase.max_shapes_drawn),
            (true, 3, Some(30))
        );
    }

    #[test]
    fn the_idle_profile_waits_out_its_length_and_steps_nothing() {
        let start = Instant::now();
        let idle = specular_bench::IDLE.with_duration(Duration::from_millis(400));
        let mut run = new_run(vec![idle], true, start);
        let mut camera = Camera::default();
        let begun = start + Duration::from_secs(2);
        assert_eq!(
            run.tick(begun, &mut camera, Vec2::ZERO),
            BenchTick::Continue
        );
        // Nothing to do until the profile ends, however often it is asked.
        assert_eq!(run.waits_until(), Some(begun + Duration::from_millis(400)));
        run.on_skipped();
        let end = begun + Duration::from_millis(400);
        run.tick(end, &mut camera, Vec2::ZERO);
        let work = run.reports()[0].work.unwrap();
        assert_eq!((work.frames_drawn, work.frames_skipped), (0, 1));
        assert_eq!(camera, Camera::default());
    }

    #[test]
    fn frames_keep_coming_before_a_profile_and_between_profiles_but_not_in_idle() {
        let start = Instant::now();
        let idle = specular_bench::IDLE.with_duration(Duration::from_millis(400));
        let mut run = new_run(vec![pan_profile(), idle], true, start);
        let warmup = Duration::from_secs(2);
        assert!(!run.keeps_presenting(start));
        assert!(run.keeps_presenting(start + Duration::from_millis(1_500)));
        let mut camera = Camera::default();
        let mut now = start + warmup;
        // Through the pan and into the gap after it.
        while run.waits_until().is_none() || now == start + warmup {
            run.tick(now, &mut camera, Vec2::ZERO);
            now += STEP;
        }
        assert!(run.keeps_presenting(now));
        // Into the idle profile, where presenting would defeat the point.
        run.tick(now + specular_bench::PHASE_GAP, &mut camera, Vec2::ZERO);
        assert!(!run.keeps_presenting(now + specular_bench::PHASE_GAP));
    }
}
