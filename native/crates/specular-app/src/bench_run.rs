//! `--bench`: drives the camera through gesture profiles and reports
//! presented-frame timing, mirroring Electron's `POST /perf/pan-zoom/run`.
//!
//! One profile step is applied per rendered frame (the Electron test steps
//! once per display refresh too), from the same start camera each time, and
//! zooms anchor at the viewport centre.

use std::time::{Duration, Instant};

use glam::Vec2;
use serde::Serialize;
use specular_bench::{
    GestureProfile, GestureStep, PHASE_GAP, PhaseRecorder, PhaseReport, PresentedFrame, build_steps,
};
use specular_compositor::{FrameObserver, FrameSample};
use specular_core::Camera;

/// One profile's result, printed as a JSON line: the bench crate's
/// [`PhaseReport`] fields at the top level, plus what only this shell knows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileReport {
    #[serde(flatten)]
    pub(crate) phase: PhaseReport,
    pub(crate) label: &'static str,
    pub(crate) source: &'static str,
    pub(crate) pages: usize,
    /// False when any frame drawn came from CPU upload: such runs must not
    /// be compared against Electron (ADR 0038).
    pub(crate) representative: bool,
    pub(crate) step_interval_ms: f64,
    pub(crate) max_paint_to_submit_ms: Option<f64>,
}

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
}

/// What the app should do after [`BenchRun::tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BenchTick {
    /// Keep rendering.
    Continue,
    /// Every profile has run; exit.
    Finished,
}

/// The benchmark state machine.
#[derive(Debug)]
pub(crate) struct BenchRun {
    profiles: Vec<GestureProfile>,
    step_interval: Duration,
    start_camera: Camera,
    source: &'static str,
    pages: usize,
    phase: Phase,
    steps: Vec<GestureStep>,
    recording: Option<Recording>,
    reports: Vec<ProfileReport>,
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
        source: &'static str,
        pages: usize,
        now: Instant,
    ) -> Self {
        Self {
            profiles,
            step_interval,
            start_camera,
            source,
            pages,
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
    pub(crate) fn reports(&self) -> &[ProfileReport] {
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
        });
        self.phase = Phase::Running {
            profile: index,
            step: 0,
        };
    }

    fn finish(&mut self, index: usize, now: Instant) {
        if let (Some(profile), Some(recording)) = (self.profiles.get(index), self.recording.take())
        {
            let representative = self.source != "synthetic" && !recording.frames.saw_cpu_texture();
            self.reports.push(ProfileReport {
                phase: recording.frames.finish(self.step_interval, None),
                label: profile.label,
                source: self.source,
                pages: self.pages,
                representative,
                step_interval_ms: millis(self.step_interval),
                max_paint_to_submit_ms: recording.max_paint_to_submit.map(millis),
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
        recording.frames.presented(
            sample.presented_at,
            PresentedFrame {
                pages_without_texture: sample.stats.pages_without_texture,
                cpu_textures: sample.stats.cpu_textures,
            },
        );
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
                interval: None,
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

    fn new_run(profiles: Vec<GestureProfile>, source: &'static str, start: Instant) -> BenchRun {
        BenchRun::new(
            profiles,
            Duration::from_secs(2),
            STEP,
            Camera::default(),
            source,
            4,
            start,
        )
    }

    #[test]
    fn run_reports_each_profile_once() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile(), pan_profile()], "cef", start);
        run_to_completion(&mut run, start, 0);
        assert_eq!(run.reports().len(), 2);
    }

    #[test]
    fn profile_applies_its_total_pan() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], "cef", start);
        let camera = run_to_completion(&mut run, start, 0);
        assert!((camera.pan.x - 30.0).abs() < 1e-4);
    }

    #[test]
    fn frames_recorded_are_intervals_between_profile_frames() {
        // Three steps -> four frames presented while running -> three intervals.
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], "cef", start);
        run_to_completion(&mut run, start, 0);
        assert_eq!(run.reports()[0].phase.frames.frames, 3);
    }

    #[test]
    fn cpu_textures_make_report_non_representative() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], "cef", start);
        run_to_completion(&mut run, start, 1);
        assert!(!run.reports()[0].representative);
    }

    #[test]
    fn synthetic_source_is_never_representative() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], "synthetic", start);
        run_to_completion(&mut run, start, 0);
        assert!(!run.reports()[0].representative);
    }

    #[test]
    fn camera_holds_still_during_warmup() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], "cef", start);
        let mut camera = Camera::new(Vec2::new(5.0, 5.0), 1.0);
        run.tick(start + Duration::from_millis(100), &mut camera, Vec2::ZERO);
        assert_eq!(camera.pan, Vec2::new(5.0, 5.0));
    }

    #[test]
    fn report_serializes_phase_fields_at_top_level() {
        let start = Instant::now();
        let mut run = new_run(vec![pan_profile()], "cef", start);
        run_to_completion(&mut run, start, 0);
        let json = serde_json::to_value(&run.reports()[0]).unwrap();
        assert_eq!(
            (&json["phase"], &json["draws"]),
            (&"slow-pan".into(), &3.into())
        );
    }
}
