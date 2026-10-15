//! What the examples share: a headless device, a readback and a blend.

use anyhow::{Context as _, Result};
use image::RgbaImage;
use specular_prism::PrismRenderer;

/// Straight-alpha `avatar` over an opaque `bg`, in the encoded values.
pub(crate) fn over(avatar: &RgbaImage, bg: [u8; 3]) -> RgbaImage {
    let mut out = avatar.clone();
    for pixel in out.pixels_mut() {
        let a = f32::from(pixel[3]) / 255.0;
        for i in 0..3 {
            pixel[i] = (f32::from(pixel[i]) * a + f32::from(bg[i]) * (1.0 - a)).round() as u8;
        }
        pixel[3] = 255;
    }
    out
}

pub(crate) fn render(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prism: &mut PrismRenderer,
    t: f32,
) -> Result<RgbaImage> {
    let size = prism.options().size;
    let stride = (size * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(stride * size),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let texture = prism.render(&mut encoder, queue, t);
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely())?;
    let data = buffer.slice(..).get_mapped_range()?;
    let bytes: Vec<u8> = data
        .chunks_exact(stride as usize)
        .flat_map(|line| &line[..(size * 4) as usize])
        .copied()
        .collect();
    RgbaImage::from_raw(size, size, bytes).context("short readback")
}

pub(crate) async fn gpu() -> Result<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await?;
    Ok(adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await?)
}
