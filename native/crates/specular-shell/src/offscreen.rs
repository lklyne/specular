//! An offscreen texture to draw the canvas into, and its PNG: what a
//! headless run and an API screenshot read back.

use std::io::Cursor;
use std::path::Path;
use std::sync::mpsc;

use anyhow::Context as _;
use specular_compositor::GpuContext;

/// Not an sRGB format, as the window's surface is not: colours blend
/// encoded, as a browser's do. The bytes are in PNG order.
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// A render target that can be read back.
#[derive(Debug)]
pub struct Target {
    texture: wgpu::Texture,
    width: u32,
    height: u32,
}

impl Target {
    /// A target of `width` by `height` device pixels in `format`, which
    /// must be the format the compositor drawing into it was made for.
    pub fn new(gpu: &GpuContext, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
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
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        Self {
            texture,
            width,
            height,
        }
    }

    /// A view to render into.
    pub fn view(&self) -> wgpu::TextureView {
        self.texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    /// Reads the texture back and writes it to `path` as a PNG.
    pub fn save(&self, gpu: &GpuContext, path: &Path) -> anyhow::Result<()> {
        let png = self.png(gpu)?;
        std::fs::write(path, png).with_context(|| format!("writing {}", path.display()))
    }

    /// The size in device pixels.
    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Reads the texture back as a PNG.
    pub fn png(&self, gpu: &GpuContext) -> anyhow::Result<Vec<u8>> {
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
        let mut rgba: Vec<u8> = data
            .chunks_exact(padded as usize)
            .flat_map(|row| &row[..row_bytes as usize])
            .copied()
            .collect();
        // A window's surface is usually BGRA; a PNG is RGBA.
        if matches!(
            self.texture.format(),
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        ) {
            for pixel in rgba.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
            }
        }
        let mut png = Cursor::new(Vec::new());
        image::write_buffer_with_format(
            &mut png,
            &rgba,
            self.width,
            self.height,
            image::ExtendedColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .context("encoding the PNG")?;
        Ok(png.into_inner())
    }
}
