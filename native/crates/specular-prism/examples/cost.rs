//! Times Prism frames on the GPU at avatar sizes.
//!
//! ```sh
//! cargo run -p specular-prism --example cost
//! ```
//!
//! Submits each frame on its own and waits for the last, so the figure is
//! the per-frame cost when frames queue back to back. Debug build: the CPU
//! side is slower than in release, the GPU side is the same.

use std::time::Instant;

use anyhow::Result;
use specular_prism::{PrismOptions, PrismRenderer};

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
    for (size, supersample) in [(48, 4), (64, 4), (64, 3), (64, 1), (128, 4)] {
        let built = Instant::now();
        let mut prism = PrismRenderer::new(&device, &queue, PrismOptions::new(size, supersample));
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
        println!("{size}px x{supersample}: {per:?} a frame ({FRAMES} frames), built in {build:?}");
    }
    Ok(())
}
