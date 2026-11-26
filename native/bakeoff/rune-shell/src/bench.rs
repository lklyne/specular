//! `--bench PROFILE`: the Kit shell's window benchmark (`specular --bench`,
//! `specular_shell::bench_drive`) under Rune, for step S6.
//!
//! Same profiles, start camera, warmup, lead-in and gap as the Kit's, and
//! the same stepping rule: one profile step a presented frame, the camera
//! moved through `update` as `Action::SetCamera`, the first page selected.
//! Intervals go through `specular_bench::PhaseRecorder`, so the percentiles
//! are computed by the code that computes the Kit's.
//!
//! Each profile prints one JSON line on stdout: the Kit's `PhaseReport`
//! fields, plus the process's CPU over the profile, the main thread's own
//! CPU time a frame, our render's CPU, Rune's `update_ms`, and how Rune
//! paced the frames (display link or not) as `SurfaceStats` said each frame.

use std::time::{Duration, Instant};

use glam::Vec2;
use specular_bench::{
    GestureProfile, GestureStep, PHASE_GAP, PhaseRecorder, PresentedFrame, ProfileId, build_steps,
    process_cpu_time,
};
use specular_compositor::SceneStats;
use specular_core::Camera;
use specular_doc::ItemId;
use specular_interact::{Action, Event};

use crate::shell::Shell;

/// The Kit bench's start camera (`specular_shell::headless::START_CAMERA`).
const START_CAMERA: Camera = Camera {
    pan: Vec2::new(40.0, 40.0),
    zoom: 0.25,
};
/// How long before a profile frames are presented anyway, so a ProMotion
/// display has ramped back up (the Kit's `LEAD_IN`).
const LEAD_IN: Duration = Duration::from_millis(500);
/// A 120 Hz display's refresh, which the Kit steps at on this Mac.
const STEP: Duration = Duration::from_nanos(8_333_333);

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    Warmup { until: Instant },
    Running { profile: usize, step: usize },
    Idle { profile: usize, until: Instant },
    Gap { until: Instant, next: usize },
    Done,
}

/// The main thread's CPU time so far. Called from Rune's handlers, which
/// all run on the main thread.
fn thread_cpu() -> Duration {
    let mut spec = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `spec` is a valid out pointer for one `timespec`.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut spec) };
    Duration::new(spec.tv_sec as u64, spec.tv_nsec as u32)
}

#[derive(Debug)]
struct Recording {
    frames: PhaseRecorder,
    began: Instant,
    cpu_began: Option<Duration>,
    thread_began: Duration,
    render_ms: Vec<f64>,
    update_ms: Vec<f64>,
    /// Frames by what paced them, as `SurfaceStats::pacing` printed it.
    pacing: std::collections::BTreeMap<String, u32>,
    presents_began: Option<u64>,
    /// With `RUNE_SHELL_GPU_TIME=1`: our pass from its submit to the GPU
    /// finishing it, each frame.
    gpu_ms: Vec<f64>,
}

/// The run.
#[derive(Debug)]
pub struct Bench {
    profiles: Vec<GestureProfile>,
    phase: Phase,
    steps: Vec<GestureStep>,
    recording: Option<Recording>,
    lines: Vec<String>,
    selected: bool,
    /// The poll has asked to be woken at the end of the current wait. A
    /// wait that begins inside a frame owes one more frame, so that a poll
    /// runs and asks: with still pages nothing else would wake the loop.
    armed: bool,
}

impl Bench {
    /// `profiles` after `warmup`, from now.
    pub fn new(profiles: Vec<GestureProfile>, warmup: Duration) -> Self {
        Self {
            profiles,
            phase: Phase::Warmup {
                until: Instant::now() + warmup,
            },
            steps: Vec::new(),
            recording: None,
            lines: Vec::new(),
            selected: false,
            armed: false,
        }
    }

    /// Whether every profile has run.
    pub fn done(&self) -> bool {
        self.phase == Phase::Done
    }

    /// When the run next has something to do with no frame drawn, or `None`
    /// while frames are owed.
    pub fn waits_until(&self, now: Instant) -> Option<Instant> {
        match self.phase {
            Phase::Warmup { until } if now + LEAD_IN < until => Some(until - LEAD_IN),
            Phase::Idle { until, .. } => Some(until),
            Phase::Warmup { .. } | Phase::Gap { .. } | Phase::Running { .. } | Phase::Done => None,
        }
    }

    /// Whether frames are owed now: the lead-in, a profile, a gap, and one
    /// more once every profile has run, whose poll prints and closes.
    pub fn animating(&self, now: Instant) -> bool {
        self.waits_until(now).is_none() || !self.armed
    }

    /// The poll asked to be woken when the current wait ends.
    pub fn arm(&mut self) {
        self.armed = true;
    }

    /// `AppSpec::poll`: ends waits whose time has come. Returns whether a
    /// frame is wanted.
    pub fn poll(&mut self, shell_now: Instant) -> bool {
        match self.phase {
            Phase::Idle { profile, until } if shell_now >= until => {
                self.finish(profile, shell_now, None);
                true
            }
            Phase::Idle { .. } => false,
            _ => self.animating(shell_now),
        }
    }

    /// Before a frame is drawn: steps the profile, moving the camera.
    pub fn before_frame(&mut self, shell: &mut Shell, now: Instant) {
        if !self.selected {
            // The Kit's bench selects the first page, so the outline and
            // handles are in every measured frame.
            self.selected = true;
            let first = shell.app.pages().next().map(|(id, ..)| id.clone());
            let selection = first.into_iter().map(ItemId::Entity).collect();
            shell.dispatch(Event::Action(Action::Select(selection)));
            // `RUNE_SHELL_POPOVER=1`: the zoom list open over the canvas
            // for the whole run, as question 9 asks.
            if std::env::var_os("RUNE_SHELL_POPOVER").is_some() {
                let zoom = specular_interact::toolbar(&shell.app)
                    .sections
                    .iter()
                    .find_map(|section| match section {
                        specular_interact::ToolbarSection::Zoom(zoom) => Some(zoom.id.clone()),
                        specular_interact::ToolbarSection::Tools(_) => None,
                    });
                shell.open_list = zoom;
            }
        }
        match self.phase {
            Phase::Warmup { until } if now >= until => self.start(0, shell, now),
            Phase::Gap { until, next } if now >= until => self.start(next, shell, now),
            Phase::Running { profile, step } => {
                if let Some(next) = self.steps.get(step).copied() {
                    let anchor = shell.viewport / 2.0;
                    let mut camera = shell.app.session().camera;
                    camera.apply_input_delta(next.to_input(Some(anchor)));
                    shell.act(Action::SetCamera(camera));
                    self.phase = Phase::Running {
                        profile,
                        step: step + 1,
                    };
                } else {
                    self.finish(profile, now, Some(shell));
                }
            }
            _ => {}
        }
    }

    /// Our pass's GPU time this frame, when it is being timed.
    pub fn gpu(&mut self, ms: f64) {
        if let Some(recording) = self.recording.as_mut() {
            recording.gpu_ms.push(ms);
        }
    }

    /// After a frame was drawn.
    pub fn after_frame(
        &mut self,
        started: Instant,
        stats: &SceneStats,
        (render_ms, update_ms): (f64, f64),
        pacing: Option<(String, u64)>,
    ) {
        let Some(recording) = self.recording.as_mut() else {
            return;
        };
        let render = &stats.render;
        recording.frames.presented(
            started,
            PresentedFrame {
                pages_without_texture: render.pages_without_texture,
                cpu_textures: render.cpu_textures,
                frames_received: render.frames_received,
                popup_frames: render.popup_frames,
                frames_dropped_for_pool_pressure: render.frames_dropped_for_pool_pressure,
                outstanding_textures: render.outstanding_textures,
                max_outstanding_textures: render.max_outstanding_textures,
            },
        );
        recording.render_ms.push(render_ms);
        recording.update_ms.push(update_ms);
        if let Some((pacing, presents)) = pacing {
            *recording.pacing.entry(pacing).or_default() += 1;
            recording.presents_began.get_or_insert(presents);
        }
    }

    fn start(&mut self, index: usize, shell: &mut Shell, now: Instant) {
        let Some(profile) = self.profiles.get(index) else {
            self.phase = Phase::Done;
            return;
        };
        shell.act(Action::SetCamera(START_CAMERA));
        eprintln!(
            "rune-shell: bench profile {} starts, viewport {:?} scale {}",
            profile.id, shell.viewport, shell.scale
        );
        self.steps = build_steps(profile, STEP);
        // Marks the profile in the ledger, so a reader can cut its window.
        specular_core::ledger::record(specular_core::ledger::Entry::Step(1));
        self.recording = Some(Recording {
            frames: PhaseRecorder::new(profile.id),
            began: now,
            cpu_began: process_cpu_time(),
            thread_began: thread_cpu(),
            render_ms: Vec::new(),
            update_ms: Vec::new(),
            pacing: Default::default(),
            presents_began: None,
            gpu_ms: Vec::new(),
        });
        self.armed = false;
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

    fn finish(&mut self, index: usize, now: Instant, _shell: Option<&mut Shell>) {
        let (Some(profile), Some(recording)) = (self.profiles.get(index), self.recording.take())
        else {
            return;
        };
        specular_core::ledger::record(specular_core::ledger::Entry::Step(2));
        let wall = now.saturating_duration_since(recording.began).as_secs_f64();
        let process_cpu = (recording.cpu_began.zip(process_cpu_time()))
            .map(|(began, ended)| ended.saturating_sub(began).as_secs_f64() / wall.max(1e-9));
        let thread_ms = (thread_cpu().saturating_sub(recording.thread_began)).as_secs_f64() * 1e3;
        let drawn = recording.render_ms.len().max(1) as f64;
        let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len().max(1) as f64;
        let p95 = |values: &[f64]| {
            let mut sorted = values.to_vec();
            sorted.sort_by(f64::total_cmp);
            sorted
                .get(((sorted.len() as f64 - 1.0) * 0.95).round().max(0.0) as usize)
                .copied()
                .unwrap_or(0.0)
        };
        let phase = recording.frames.finish(STEP);
        let mut line = serde_json::to_value(phase).unwrap_or_default();
        if let Some(object) = line.as_object_mut() {
            let extra = serde_json::json!({
                "label": profile.label,
                "shell": "rune",
                "wallMs": wall * 1e3,
                "framesDrawn": recording.render_ms.len(),
                "processCpu": process_cpu,
                "mainThreadCpu": thread_ms / (wall * 1e3).max(1e-9),
                "mainThreadCpuMsPerFrame": thread_ms / drawn,
                "renderCpuMeanMs": mean(&recording.render_ms),
                "renderCpuP95Ms": p95(&recording.render_ms),
                "runeUpdateMeanMs": mean(&recording.update_ms),
                "pacing": recording.pacing,
                "gpuMeanMs": (!recording.gpu_ms.is_empty()).then(|| mean(&recording.gpu_ms)),
                "gpuP95Ms": (!recording.gpu_ms.is_empty()).then(|| p95(&recording.gpu_ms)),
            });
            if let Some(extra) = extra.as_object() {
                object.extend(extra.clone());
            }
        }
        self.lines.push(line.to_string());
        self.phase = Phase::Gap {
            until: now + PHASE_GAP,
            next: index + 1,
        };
    }

    /// Prints the lines, once every profile has run.
    pub fn print(&mut self) {
        for line in self.lines.drain(..) {
            println!("{line}");
        }
    }
}
