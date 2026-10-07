//! The offscreen texture a headless run draws into, and its PNG.

use std::path::Path;
use std::sync::mpsc;

use anyhow::Context as _;
use specular_compositor::GpuContext;

/// Not an sRGB format, as the window's surface is not: colours blend
/// encoded, as a browser's do. The bytes are in PNG order.
pub(super) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// A render target that can be read back.
pub(super) struct Target {
    texture: wgpu::Texture,
    width: u32,
    height: u32,
}

impl Target {
    pub(super) fn new(gpu: &GpuContext, width: u32, height: u32) -> Self {
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("snapshot-target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self {
            texture,
            width,
            height,
        }
    }

    pub(super) fn view(&self) -> wgpu::TextureView {
        self.texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Reads the texture back and writes it to `path` as a PNG.
    pub(super) fn save(&self, gpu: &GpuContext, path: &Path) -> anyhow::Result<()> {
        let row_bytes = self.width * 4;
        let padded = row_bytes.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
            * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("snapshot-readback"),
            size: u64::from(padded) * u64::from(self.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            self.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: None,
                },
            },
            self.texture.size(),
        );
        gpu.queue.submit([encoder.finish()]);
        let (mapped_tx, mapped) = mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                // The receiver outlives the poll below.
                let _ = mapped_tx.send(result);
            });
        gpu.device
            .poll(wgpu::PollType::wait_indefinitely())
            .context("waiting for the snapshot readback")?;
        mapped
            .recv()
            .context("the readback was never mapped")?
            .context("mapping the snapshot readback")?;
        let data = buffer
            .slice(..)
            .get_mapped_range()
            .context("reading the snapshot readback")?;
        let rgba: Vec<u8> = data
            .chunks_exact(padded as usize)
            .flat_map(|row| &row[..row_bytes as usize])
            .copied()
            .collect();
        image::save_buffer(
            path,
            &rgba,
            self.width,
            self.height,
            image::ExtendedColorType::Rgba8,
        )
        .with_context(|| format!("writing {}", path.display()))
    }
}
