//! The run every candidate goes through: golden images, then timed camera paths.

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use anyhow::Result;

use crate::{
    Camera, Gpu, World,
    camera::{GOLDEN_ZOOMS, VIEWPORT},
    gpu::write_pngs,
};

const SWEEP_FRAMES: usize = 600;
const PAN_FRAMES: usize = 120;
const WARMUP_FRAMES: usize = 8;

/// One renderer under test.
pub trait Candidate {
    /// Encodes and submits one frame into the candidate's own target.
    fn frame(&mut self, gpu: &Gpu, world: &World, camera: Camera) -> Result<()>;
    /// The `Rgba8Unorm` texture `frame` drew into.
    fn target(&self) -> &wgpu::Texture;
}

/// Whether a command-line flag such as `--retained` was passed.
pub fn flag(name: &str) -> bool {
    std::env::args().any(|arg| arg == name)
}

/// The run's name: the candidate plus each mode flag that is on.
pub fn run_name(candidate: &str) -> String {
    let modes = ["--retained", "--lod"]
        .into_iter()
        .filter(|mode| flag(mode));
    modes.fold(candidate.to_owned(), |name, mode| {
        name + "-" + mode.trim_start_matches('-')
    })
}

/// Where the golden images for this run go: `--out <dir>`, default `out`.
pub fn out_dir() -> PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--out" {
            return args
                .next()
                .map_or_else(|| PathBuf::from("out"), PathBuf::from);
        }
    }
    PathBuf::from("out")
}

pub fn golden_images(
    name: &str,
    gpu: &Gpu,
    world: &World,
    candidate: &mut dyn Candidate,
) -> Result<()> {
    let dir = out_dir();
    for zoom in GOLDEN_ZOOMS {
        candidate.frame(gpu, world, Camera::golden(world, zoom))?;
        let rgba = gpu.read_rgba(candidate.target())?;
        write_pngs(&rgba, &dir, &format!("{name}-zoom-{zoom}"))?;
    }
    Ok(())
}

/// Draws `panels` over a zoom-1 frame, saves it, and times the extra pass.
pub fn panel_overlay(
    name: &str,
    gpu: &Gpu,
    world: &World,
    candidate: &mut dyn Candidate,
    mut panels: impl FnMut(&Gpu, &wgpu::TextureView, f32),
) -> Result<()> {
    let camera = Camera::golden(world, 1.0);
    let view = candidate
        .target()
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut cpu = Vec::with_capacity(PAN_FRAMES);
    let mut total = Vec::with_capacity(PAN_FRAMES);
    for _ in 0..PAN_FRAMES {
        candidate.frame(gpu, world, camera)?;
        gpu.wait()?;
        let start = Instant::now();
        panels(gpu, &view, camera.zoom);
        cpu.push(start.elapsed());
        gpu.wait()?;
        total.push(start.elapsed());
    }
    write_pngs(
        &gpu.read_rgba(candidate.target())?,
        &out_dir(),
        &format!("{name}-egui"),
    )?;
    let [cpu, total] = [summary(&mut cpu), summary(&mut total)];
    println!(
        "\negui panels over {name}: cpu mean {:.2} p95 {:.2} max {:.2}, total mean {:.2} p95 {:.2} max {:.2} ms",
        cpu[0], cpu[1], cpu[2], total[0], total[1], total[2]
    );
    Ok(())
}

/// Times the zoom sweep and a pan at each golden zoom, printing one table row each.
pub fn timings(name: &str, gpu: &Gpu, world: &World, candidate: &mut dyn Candidate) -> Result<()> {
    println!(
        "\n{name} on {}, {}x{} offscreen, ms per frame",
        gpu.adapter, VIEWPORT[0], VIEWPORT[1]
    );
    println!("| path | cpu mean | cpu p95 | cpu max | total mean | total p95 | total max |");
    println!("|---|---|---|---|---|---|---|");
    let sweep: Vec<Camera> = Camera::sweep(world, SWEEP_FRAMES).collect();
    time_path("sweep 0.02-3", gpu, world, candidate, &sweep)?;
    for zoom in GOLDEN_ZOOMS {
        let pan: Vec<Camera> = Camera::pan(world, zoom, PAN_FRAMES).collect();
        time_path(&format!("pan at {zoom}"), gpu, world, candidate, &pan)?;
    }
    Ok(())
}

fn time_path(
    label: &str,
    gpu: &Gpu,
    world: &World,
    candidate: &mut dyn Candidate,
    path: &[Camera],
) -> Result<()> {
    for camera in &path[..WARMUP_FRAMES.min(path.len())] {
        candidate.frame(gpu, world, *camera)?;
        gpu.wait()?;
    }
    let mut cpu = Vec::with_capacity(path.len());
    let mut total = Vec::with_capacity(path.len());
    for camera in path {
        let start = Instant::now();
        candidate.frame(gpu, world, *camera)?;
        cpu.push(start.elapsed());
        gpu.wait()?;
        total.push(start.elapsed());
    }
    let [cpu, total] = [summary(&mut cpu), summary(&mut total)];
    println!(
        "| {label} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |",
        cpu[0], cpu[1], cpu[2], total[0], total[1], total[2]
    );
    Ok(())
}

/// Mean, 95th percentile and max, in milliseconds.
fn summary(samples: &mut [Duration]) -> [f64; 3] {
    samples.sort_unstable();
    let ms = |d: Duration| d.as_secs_f64() * 1e3;
    let mean = samples.iter().copied().map(ms).sum::<f64>() / samples.len() as f64;
    [
        mean,
        ms(samples[samples.len() * 95 / 100]),
        ms(samples[samples.len() - 1]),
    ]
}
