//! `--bench --bench-target headless`: the gesture profiles drawn into a
//! texture with no window and no vsync, so a frame's cost is not hidden
//! behind the display's refresh.
//!
//! Each step moves the camera, builds the scene, draws it and waits for
//! the GPU to finish, timing every part. Steps are paced at the display's
//! interval so pages paint as they would between real frames.

use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use glam::Vec2;
use specular_bench::{
    FrameWork, GestureProfile, PaintPolicy, PhaseRecorder, PresentedFrame, ProfileLine,
    WorkRecorder, build_steps,
};
use specular_compositor::{DotGrid, FrameView, SceneStats};
use specular_core::{Camera, PageSource};
use specular_doc::Document;
use specular_interact::{Action, Event};

use super::{Headless, HeadlessArgs};

/// The interval a 120 Hz display steps a gesture at.
const STEP: Duration = Duration::from_nanos(8_333_333);

/// What a headless benchmark runs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BenchPlan {
    pub(crate) profiles: Vec<GestureProfile>,
    /// The camera each profile starts from.
    pub(crate) start: Camera,
    /// Whether the session layer is drawn.
    pub(crate) chrome: bool,
}

/// Opens `document`, runs every profile and prints one JSON line each.
pub(crate) fn run(
    source: Box<dyn PageSource>,
    document: Document,
    canvas: Option<&Path>,
    args: &HeadlessArgs,
    plan: &BenchPlan,
) -> anyhow::Result<()> {
    let mut run = Headless::new(source, canvas, args)?;
    let viewport = run.viewport;
    run.drive(|app| {
        app.viewport(viewport)
            .send(Event::DocumentOpened(Box::new(document)))
    })?;
    run.settle()?;
    let name = canvas
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned());
    for profile in &plan.profiles {
        let line = run.profile(profile, plan, name.clone())?;
        println!("{}", serde_json::to_string(&line)?);
    }
    run.source.shutdown();
    Ok(())
}

impl Headless {
    fn profile(
        &mut self,
        profile: &GestureProfile,
        plan: &BenchPlan,
        canvas: Option<String>,
    ) -> anyhow::Result<ProfileLine> {
        let mut camera = plan.start;
        self.drive(|app| app.act(Action::SetCamera(camera)))?;
        // One frame outside the record, so the profile starts warm as a
        // gesture on an open canvas does.
        self.frame(plan.chrome, false)?;
        let mut frames = PhaseRecorder::new(profile.id);
        let mut work = WorkRecorder::new();
        let mut drawn_zoom = camera.zoom;
        let mut next = Instant::now();
        for step in build_steps(profile, STEP) {
            let started = Instant::now();
            let before = camera;
            camera.apply_input_delta(step.to_input(Some(self.viewport / 2.0)));
            if camera != before {
                self.drive(|app| app.act(Action::SetCamera(camera)))?;
            }
            let update = started.elapsed();
            let zooming = (camera.zoom - drawn_zoom).abs() > f32::EPSILON;
            drawn_zoom = camera.zoom;
            let (mut frame, stats) = self.frame(plan.chrome, zooming)?;
            frame.update_ms = millis(update);
            work.drawn(frame);
            frames.presented(Instant::now(), presented(&stats));
            self.report_note_heights()?;
            next += STEP;
            self.take_page_events()?;
            if let Some(wait) = next.checked_duration_since(Instant::now()) {
                std::thread::sleep(wait);
            } else {
                next = Instant::now();
            }
        }
        Ok(ProfileLine {
            phase: frames.finish(STEP),
            label: profile.label.to_owned(),
            source: self.source.name().to_owned(),
            pages: self.hosts.len(),
            // No presented frame at all, so nothing to compare with Electron.
            representative: false,
            step_interval_ms: millis(STEP),
            max_paint_to_submit_ms: None,
            paint_policy: PaintPolicy::FullRate,
            chrome: plan.chrome,
            annotations: 0,
            target: Some("headless".to_owned()),
            shell: None,
            canvas,
            work: Some(work.finish()),
        })
    }

    /// Draws one frame and waits for the GPU to finish it.
    fn frame(&mut self, chrome: bool, zooming: bool) -> anyhow::Result<(FrameWork, SceneStats)> {
        let started = Instant::now();
        // No panels: a benchmark measures the canvas, in a window too.
        let scene = if chrome {
            specular_scene::view(self.app.app(), self.viewport, &self.view_cache)
        } else {
            specular_scene::view_without_chrome(self.app.app(), self.viewport, &self.view_cache)
        };
        let view = started.elapsed();
        let frame = FrameView {
            camera: self.app.session().camera,
            viewport: self.viewport,
            scale_factor: self.scale,
            grid: DotGrid::default(),
            zooming,
        };
        let hosts = &self.hosts;
        let stats = self
            .compositor
            .render_scene(&self.target.view(), &frame, &scene, |entity| {
                hosts.get(entity).copied()
            });
        let submitted = Instant::now();
        self.gpu
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .context("waiting for the frame")?;
        let work = FrameWork {
            view_ms: millis(view),
            gpu_ms: millis(submitted.elapsed()),
            ..work_of(&stats)
        };
        Ok((work, stats))
    }
}

/// The compositor's share of a frame's work.
pub(crate) fn work_of(stats: &SceneStats) -> FrameWork {
    let times = &stats.times;
    FrameWork {
        cull_ms: millis(times.cull),
        shaping_ms: millis(times.shaping),
        batching_ms: millis(times.batching),
        tessellation_ms: millis(times.tessellation),
        build_ms: millis(times.build),
        glyphs_ms: millis(times.glyphs),
        upload_ms: millis(times.upload),
        submit_ms: millis(times.submit),
        items: stats.items_drawn,
        batches: stats.batches,
        draw_calls: stats.draw_calls,
        glyphs: stats.glyphs,
        triangles: stats.triangles,
        ..FrameWork::default()
    }
}

fn presented(stats: &SceneStats) -> PresentedFrame {
    let render = &stats.render;
    PresentedFrame {
        pages_without_texture: render.pages_without_texture,
        cpu_textures: render.cpu_textures,
        frames_received: render.frames_received,
        popup_frames: render.popup_frames,
        frames_dropped_for_pool_pressure: render.frames_dropped_for_pool_pressure,
        outstanding_textures: render.outstanding_textures,
        max_outstanding_textures: render.max_outstanding_textures,
    }
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

/// The start camera a bench uses when the command line names none.
pub(crate) const START_CAMERA: Camera = Camera {
    pan: Vec2::new(40.0, 40.0),
    zoom: 0.25,
};
