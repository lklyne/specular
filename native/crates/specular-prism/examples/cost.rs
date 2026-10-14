//! Times Prism frames on the GPU at avatar sizes.
//!
//! ```sh
//! cargo run -p specular-prism --example cost
//! ```
//!
//! Submits each frame on its own and waits for the last, so the first
//! figure is the per-frame cost when frames queue back to back. The second
//! submits them all at once, which leaves mostly the GPU's own work. Debug build: the CPU
//! side is slower than in release, the GPU side is the same.

use std::time::Instant;

use anyhow::Result;
use specular_prism::{PrismOptions, PrismRenderer, SHELL_SEGMENTS};

const FRAMES: u32 = 300;

fn main() -> Result<()> {
    pollster::block_on(run())
}

async fn run() -> Result<()> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..wgpu::RequestAdapterOptions::default()
        })
        .await?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await?;
    // (size, supersample, shell segments): the web build's subdivision, then
    // the Kit avatar's (40 px at 3x, 24 segments).
    for (size, supersample, segments) in [
        (64, 4, SHELL_SEGMENTS),
        (64, 1, SHELL_SEGMENTS),
        (40, 3, SHELL_SEGMENTS),
        (40, 3, 24),
        (40, 4, 24),
    ] {
        let built = Instant::now();
        let options = PrismOptions {
            shell_segments: segments,
            ..PrismOptions::new(size, supersample)
        };
        let mut prism = PrismRenderer::new(&device, &queue, options);
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let build = built.elapsed();
        // One frame first, so pipeline compilation is not timed.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        prism.render(&mut encoder, &queue, 0.0);
        queue.submit([encoder.finish()]);
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let start = Instant::now();
        for i in 0..FRAMES {
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            prism.render(&mut encoder, &queue, i as f32 / 60.0);
            queue.submit([encoder.finish()]);
        }
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let per = start.elapsed() / FRAMES;
        // The same frames recorded first and then submitted at once: the
        // wait after the submit is the GPU's work with no round trip per
        // frame. Every frame sees the last frame's uniforms, which costs the
        // same to draw.
        let start = Instant::now();
        let batch: Vec<_> = (0..FRAMES)
            .map(|i| {
                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                prism.render(&mut encoder, &queue, i as f32 / 60.0);
                encoder.finish()
            })
            .collect();
        let recorded = start.elapsed() / FRAMES;
        let start = Instant::now();
        queue.submit(batch);
        device.poll(wgpu::PollType::wait_indefinitely())?;
        let gpu = start.elapsed() / FRAMES;
        println!(
            "{size}px x{supersample}, {segments} segments: {per:?} a frame submitted alone; \
             batched, {recorded:?} to record and {gpu:?} on the GPU; built in {build:?}"
        );
    }
    Ok(())
}
