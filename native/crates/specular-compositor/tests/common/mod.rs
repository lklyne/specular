//! Shared helpers for GPU integration tests. Every test calls [`gpu_or_skip`]
//! first so machines without an adapter (CI, the dev container) pass.

#![expect(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers fail the calling test loudly"
)]

use specular_compositor::{CompositorError, GpuContext};

/// Side length of the square render target used by these tests.
pub(crate) const TARGET_SIZE: u32 = 64;

/// A GPU context, or `None` (after printing why) when no adapter exists.
pub(crate) fn gpu_or_skip() -> Option<GpuContext> {
    match pollster::block_on(GpuContext::headless()) {
        Ok(gpu) => Some(gpu),
        Err(CompositorError::NoAdapter(reason)) => {
            eprintln!("skipping: no GPU adapter ({reason})");
            None
        }
        Err(other) => panic!("GPU setup failed: {other}"),
    }
}

/// A `TARGET_SIZE`² render target that can be read back.
pub(crate) fn render_target(gpu: &GpuContext, format: wgpu::TextureFormat) -> wgpu::Texture {
    gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("test-target"),
        size: wgpu::Extent3d {
            width: TARGET_SIZE,
            height: TARGET_SIZE,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Reads a 4-byte-per-texel target back as rows of texels.
pub(crate) fn read_pixels(gpu: &GpuContext, target: &wgpu::Texture) -> Vec<[u8; 4]> {
    let row_bytes = TARGET_SIZE * 4;
    let padded =
        row_bytes.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(padded * TARGET_SIZE),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        target.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: None,
            },
        },
        target.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.unwrap());
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    let data = buffer.slice(..).get_mapped_range().unwrap();
    data.chunks_exact(padded as usize)
        .flat_map(|row| row[..row_bytes as usize].as_chunks::<4>().0.iter().copied())
        .collect()
}

/// The texel at (`x`, `y`).
pub(crate) fn pixel(pixels: &[[u8; 4]], x: u32, y: u32) -> [u8; 4] {
    pixels[(y * TARGET_SIZE + x) as usize]
}
