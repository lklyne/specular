//! `--bench`: drives the camera through gesture profiles and reports
//! presented-frame timing, mirroring Electron's `POST /perf/pan-zoom/run`.
//!
//! One profile step is applied per rendered frame (the Electron test steps
//! once per display refresh too), from the same start camera each time, and
//! zooms anchor at the viewport centre.

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_bench::{
    GestureProfile, GestureStep, PHASE_GAP, PaintPolicy, PhaseRecorder, PresentedFrame,
    ProfileLine, build_steps,
};
use specular_compositor::{FrameObserver, FrameSample};
use specular_core::Camera;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Warmup { until: Instant },
    Running { profile: usize, step: usize },
    Gap { until: Instant, next: usize },
    Done,
}

#[derive(Debug)]
struct Recording {
    frames: PhaseRecorder,
    max_paint_to_submit: Option<Duration>,
    max_shapes_drawn: u32,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
            Phase::Warmup { until } | Phase::Gap { until, next: _ } if now < until => {}
            Phase::Warmup { .. } => self.start(0, camera),
            Phase::Gap { next, .. } => self.start(next, camera),
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

    /// Completed profile reports, in run order.
    pub(crate) fn reports(&self) -> &[ProfileLine] {
        &self.reports
    }

    fn start(&mut self, index: usize, camera: &mut Camera) {
        let Some(profile) = self.profiles.get(index) else {
            self.phase = Phase::Done;
            return;
        };
        *camera = self.start_camera;
        self.steps = build_steps(profile, self.step_interval);
        self.recording = Some(Recording {
            frames: PhaseRecorder::new(profile.id),
            max_paint_to_submit: None,
            max_shapes_drawn: 0,
        });
        self.phase = Phase::Running {
            profile: index,
            step: 0,
        };
    }

    fn finish(&mut self, index: usize, now: Instant) {
        if let (Some(profile), Some(recording)) = (self.profiles.get(index), self.recording.take())
        {
            let representative = self.source.representative && !recording.frames.saw_cpu_texture();
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
            },
            start,
        )
    }

    #[test]
    fn run_reports_each_profile_once() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile(), pan_profile()], true, start);
        run_to_completion(&mut run, start, 0);
        assert_eq!(run.reports().len(), 2);
    }

    #[test]
    fn profile_applies_its_total_pan() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], true, start);
        let camera = run_to_completion(&mut run, start, 0);
        assert!((camera.pan.x - 30.0).abs() < 1e-4);
    }

    #[test]
    fn frames_recorded_are_intervals_between_profile_frames() {
        // Three steps -> four frames presented while running -> three intervals.
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], true, start);
        run_to_completion(&mut run, start, 0);
        assert_eq!(run.reports()[0].phase.frames.frames, 3);
    }

    #[test]
    fn cpu_textures_make_report_non_representative() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], true, start);
        run_to_completion(&mut run, start, 1);
        assert!(!run.reports()[0].representative);
    }

    #[test]
    fn non_representative_source_is_never_representative() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], false, start);
        run_to_completion(&mut run, start, 0);
        assert!(!run.reports()[0].representative);
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
    fn report_serializes_phase_fields_at_top_level() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], true, start);
        run_to_completion(&mut run, start, 0);
        let json = serde_json::to_value(&run.reports()[0]).unwrap();
        assert_eq!(
            (&json["phase"], &json["draws"]),
            (&"slow-pan".into(), &3.into())
        );
    }
}
