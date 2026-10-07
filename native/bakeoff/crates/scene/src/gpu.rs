//! Headless wgpu device, offscreen targets and PNG readback.

use std::{fs::File, io::BufWriter, path::Path};

use anyhow::{Context, Result};

use crate::camera::VIEWPORT;

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter: String,
}

impl Gpu {
    pub fn headless() -> Result<Self> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..wgpu::RequestAdapterOptions::default()
        }))
        .context("no GPU adapter")?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("bakeoff"),
                required_limits: adapter.limits(),
                ..wgpu::DeviceDescriptor::default()
            }))?;
        let info = adapter.get_info();
        Ok(Self {
            device,
            queue,
            adapter: format!("{} ({:?})", info.name, info.backend),
        })
    }

    /// A viewport-sized texture that can also be read back.
    pub fn target(
        &self,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        sample_count: u32,
    ) -> wgpu::Texture {
        self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bakeoff-target"),
            size: wgpu::Extent3d {
                width: VIEWPORT[0],
                height: VIEWPORT[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    }

    /// Blocks until everything submitted so far has finished on the GPU.
    pub fn wait(&self) -> Result<()> {
        self.device.poll(wgpu::PollType::wait_indefinitely())?;
        Ok(())
    }

    /// Reads an `Rgba8Unorm` target back as tightly packed rows.
    pub fn read_rgba(&self, target: &wgpu::Texture) -> Result<Vec<u8>> {
        let row_bytes = VIEWPORT[0] * 4;
        let padded = row_bytes.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(padded * VIEWPORT[1]),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
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
        self.queue.submit([encoder.finish()]);
        buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
        self.wait()?;
        let data = buffer.slice(..).get_mapped_range()?;
        Ok(data
            .chunks_exact(padded as usize)
            .flat_map(|row| row[..row_bytes as usize].iter().copied())
            .collect())
    }
}

/// Writes the full frame, plus a 1:1 crop of its middle for judging text edges.
pub fn write_pngs(rgba: &[u8], dir: &Path, name: &str) -> Result<()> {
    std::fs::create_dir_all(dir)?;
    write_png(&dir.join(format!("{name}.png")), rgba, VIEWPORT)?;

    let crop = [480, 300];
    let left = (VIEWPORT[0] - crop[0]) / 2;
    let top = (VIEWPORT[1] - crop[1]) / 2;
    let cropped: Vec<u8> = (top..top + crop[1])
        .flat_map(|y| {
            let start = ((y * VIEWPORT[0] + left) * 4) as usize;
            rgba[start..start + (crop[0] * 4) as usize].iter().copied()
        })
        .collect();
    write_png(&dir.join(format!("{name}-crop.png")), &cropped, crop)
}

fn write_png(path: &Path, rgba: &[u8], size: [u32; 2]) -> Result<()> {
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path)?), size[0], size[1]);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(rgba)?;
    Ok(())
}
