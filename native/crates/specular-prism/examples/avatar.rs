//! A contact sheet of Prism avatars, for picking the framing, supersample
//! and shell subdivision by eye.
//!
//! ```sh
//! cargo run -p specular-prism --example avatar -- OUT_DIR [SIZE]
//! ```
//!
//! Each row is one setup and each column one time. Every cell is rendered
//! at `SIZE` pixels (default 40, a 20 pt slot at 2x) with straight alpha,
//! laid over a light and a dark panel colour and blown up 8x, nearest. The
//! first row is the web build's frame, Lanczos-downscaled to the same
//! framing, when `SPECULAR_PRISM_REFERENCE` names its capture (ADR 0047).

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use image::{Rgba, RgbaImage, imageops};
use specular_prism::{Alpha, PrismOptions, PrismRenderer};

const ZOOM: u32 = 8;
const LIGHT: [u8; 3] = [255, 255, 255];
const DARK: [u8; 3] = [24, 24, 27];
const TIMES: [f32; 6] = [0.0, 2.0, 4.0, 6.0, 9.0, 13.0];
/// The web build's frame shows half an edge of 0.7 world units.
const WEB_HALF_EXTENT: f32 = 0.7;

/// (half extent, supersample, shell segments)
const SETUPS: [(f32, u32, u32); 6] = [
    (0.7, 4, 80),
    (0.40, 2, 24),
    (0.40, 3, 24),
    (0.40, 4, 24),
    (0.40, 4, 80),
    (0.42, 4, 24),
];

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().context("usage: avatar OUT_DIR [SIZE]")?);
    let size: u32 = args.next().map_or(Ok(40), |s| s.parse())?;
    std::fs::create_dir_all(&out)?;
    let (device, queue) = pollster::block_on(gpu())?;

    let mut rows: Vec<Vec<RgbaImage>> = Vec::new();
    if let Some(capture) = std::env::var_os("SPECULAR_PRISM_REFERENCE") {
        let reference = Path::new(&capture).join("shots/f0240-1024-alpha.png");
        let full = image::open(&reference)
            .with_context(|| format!("reading {}", reference.display()))?
            .to_rgba8();
        let crop = (full.width() as f32 * 0.40 / WEB_HALF_EXTENT).round() as u32;
        let at = (full.width() - crop) / 2;
        let cut = imageops::crop_imm(&full, at, at, crop, crop).to_image();
        let small = imageops::resize(&cut, size, size, imageops::FilterType::Lanczos3);
        image::save_buffer(
            out.join("reference-f0240.png"),
            &small,
            size,
            size,
            image::ColorType::Rgba8,
        )?;
        rows.push(vec![small]);
    }
    for (half, supersample, segments) in SETUPS {
        let mut prism = PrismRenderer::new(
            &device,
            &queue,
            PrismOptions {
                alpha: Alpha::Straight,
                shell_segments: segments,
                ..PrismOptions::new(size, supersample)
            },
        );
        prism.params_mut().camera.half_extent = half;
        let mut row = Vec::new();
        for t in TIMES {
            row.push(render(&device, &queue, &mut prism, t)?);
        }
        let name = format!("h{half:.2}-ss{supersample}-s{segments}");
        image::save_buffer(
            out.join(format!("{name}-t4.png")),
            &row[2],
            size,
            size,
            image::ColorType::Rgba8,
        )?;
        println!("row {}: {name}", rows.len());
        rows.push(row);
    }
    let cell = size * ZOOM;
    let columns = TIMES.len() as u32 * 2;
    let mut sheet = RgbaImage::from_pixel(
        columns * (cell + 8),
        rows.len() as u32 * (cell + 8),
        Rgba([128, 128, 128, 255]),
    );
    for (r, row) in rows.iter().enumerate() {
        for (c, avatar) in row.iter().enumerate() {
            for (side, bg) in [LIGHT, DARK].into_iter().enumerate() {
                let over = over(avatar, bg);
                let big = imageops::resize(&over, cell, cell, imageops::FilterType::Nearest);
                let x = (c as u32 * 2 + side as u32) * (cell + 8);
                imageops::replace(
                    &mut sheet,
                    &big,
                    i64::from(x),
                    i64::from(r as u32 * (cell + 8)),
                );
            }
        }
    }
    sheet.save(out.join("sheet.png"))?;
    println!("columns: t = {TIMES:?}, each over light then dark");
    Ok(())
}

/// Straight-alpha `avatar` over an opaque `bg`, in the encoded values.
fn over(avatar: &RgbaImage, bg: [u8; 3]) -> RgbaImage {
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

fn render(
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

async fn gpu() -> Result<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await?;
    Ok(adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await?)
}
