//! Renders Prism frames headless and diffs every pass against the capture
//! of the web build.
//!
//! ```sh
//! cargo run -p specular-prism --example parity [REFERENCE_DIR] [OUT_DIR]
//! ```
//!
//! `REFERENCE_DIR` defaults to the capture in `.prism-work/reference` at the
//! repository root, and `OUT_DIR` to `.prism-work/parity/<unix time>`. It
//! prints the max and mean absolute difference per pass (in 1/255 steps)
//! and writes `<frame>-<pass>.png` (reference | ours | diff x8) for each.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use image::{Rgba, RgbaImage};
use specular_prism::{PrismOptions, PrismRenderer, Stage};

/// The page colour the web canvas sits on.
const BG: [f32; 3] = [233.0 / 255.0, 229.0 / 255.0, 220.0 / 255.0];

/// Each stage, and the capture's pass number that holds the same target.
const STAGES: [(Stage, &str, &str); 5] = [
    (Stage::Card, "card", "pass00"),
    (Stage::Liquid, "liquid", "pass02"),
    (Stage::Glass, "glass", "pass04"),
    (Stage::Display, "display", "pass06"),
    (Stage::Output, "output", "pass05"),
];

struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// RGBA floats, row 0 at the top.
struct Image {
    size: u32,
    texels: Vec<[f32; 4]>,
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.prism-work");
    let reference = args
        .next()
        .map_or_else(|| root.join("reference"), PathBuf::from);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let out = args
        .next()
        .map_or_else(|| root.join(format!("parity/{stamp}")), PathBuf::from);
    std::fs::create_dir_all(&out)?;
    let gpu = pollster::block_on(gpu())?;

    for frame in [0_u32, 240, 480] {
        let tag = format!("f{frame:04}");
        let t = frame as f32 / 60.0;
        let options = PrismOptions {
            format: wgpu::TextureFormat::Rgba16Float,
            ..PrismOptions::new(1024, 1)
        };
        let mut prism = PrismRenderer::new(&gpu.device, &gpu.queue, options);
        check_uniforms(
            &mut prism,
            &gpu,
            &reference.join("uniform-bytes").join(&tag),
            t,
        );
        for (stage, name, pass) in STAGES {
            let ours = render(&mut prism, &gpu, t, stage)?;
            let Some(theirs) = reference_pass(&reference.join("pass-outputs").join(&tag), pass)?
            else {
                continue;
            };
            report(&format!("{tag} {name}"), &theirs, &ours);
            side_by_side(&theirs, &ours, &out.join(format!("{tag}-{name}.png")))?;
            if stage == Stage::Output {
                let shot = load_png(&reference.join(format!("shots/{tag}-1024-over-bg.png")))?;
                let ours = over_bg(&ours);
                report(&format!("{tag} over-bg png"), &shot, &ours);
                side_by_side(&shot, &ours, &out.join(format!("{tag}-over-bg.png")))?;
            }
        }
    }

    for (size, frame) in [(128_u32, 240_u32), (64, 240), (128, 480), (64, 480)] {
        let tag = format!("f{frame:04}");
        let mut prism = PrismRenderer::new(&gpu.device, &gpu.queue, PrismOptions::new(size, 4));
        let ours = over_bg(&render(
            &mut prism,
            &gpu,
            frame as f32 / 60.0,
            Stage::Output,
        )?);
        let down = reference.join(format!("shots/{tag}-1024-downscaled-{size}-over-bg.png"));
        let theirs = load_png(&down)?;
        report(
            &format!("{tag} {size}px 4x vs downscaled 1024"),
            &theirs,
            &ours,
        );
        side_by_side(
            &theirs,
            &ours,
            &out.join(format!("{tag}-{size}-ss4-vs-downscaled.png")),
        )?;
        let native = reference.join(format!("shots/{tag}-s{size}-{size}-over-bg.png"));
        if native.exists() {
            let theirs = load_png(&native)?;
            report(
                &format!("{tag} {size}px 4x vs web at {size}"),
                &theirs,
                &ours,
            );
            side_by_side(
                &theirs,
                &ours,
                &out.join(format!("{tag}-{size}-ss4-vs-native.png")),
            )?;
        }
    }
    println!("wrote {}", out.display());
    Ok(())
}

async fn gpu() -> Result<Gpu> {
    let instance = wgpu::Instance::default();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..wgpu::RequestAdapterOptions::default()
        })
        .await?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("prism-parity"),
            required_limits: adapter.limits(),
            ..wgpu::DeviceDescriptor::default()
        })
        .await?;
    Ok(Gpu { device, queue })
}

/// Compares the packed uniforms with the bytes the web build bound, word by
/// word, and prints the members that differ by more than float noise.
fn check_uniforms(prism: &mut PrismRenderer, gpu: &Gpu, dir: &Path, t: f32) {
    for (name, ours) in prism.uniform_bytes(&gpu.queue, t) {
        let Ok(theirs) = std::fs::read(dir.join(format!("{name}.bin"))) else {
            continue;
        };
        if theirs.len() < ours.len() {
            println!(
                "uniforms {name}: capture has {} bytes, ours {}",
                theirs.len(),
                ours.len()
            );
            continue;
        }
        let words = |b: &[u8]| -> Vec<f32> { bytemuck::pod_collect_to_vec(b) };
        let (a, b) = (words(&theirs[..ours.len()]), words(&ours));
        let bad: Vec<String> = a
            .iter()
            .zip(&b)
            .enumerate()
            .filter(|(_, (x, y))| {
                !(x.to_bits() == y.to_bits()
                    || (x.is_nan() && y.is_nan())
                    || (*x - *y).abs() <= 1e-5 * x.abs().max(1.0))
            })
            .map(|(i, (x, y))| format!("@{}: {x} vs {y}", i * 4))
            .collect();
        if bad.is_empty() {
            println!(
                "uniforms {name} {}: match",
                dir.file_name().unwrap_or_default().display()
            );
        } else {
            println!(
                "uniforms {name}: {} words differ: {}",
                bad.len(),
                bad.join(", ")
            );
        }
    }
}

fn render(prism: &mut PrismRenderer, gpu: &Gpu, t: f32, stage: Stage) -> Result<Image> {
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let texture = prism
        .render_through(&mut encoder, &gpu.queue, t, stage)
        .clone();
    gpu.queue.submit([encoder.finish()]);
    read(gpu, &texture)
}

fn read(gpu: &Gpu, texture: &wgpu::Texture) -> Result<Image> {
    let size = texture.width();
    let format = texture.format();
    let texel = format.block_copy_size(None).context("texel size")?;
    let row = size * texel;
    let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(padded * size),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    gpu.queue.submit([encoder.finish()]);
    buffer.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    gpu.device.poll(wgpu::PollType::wait_indefinitely())?;
    let data = buffer.slice(..).get_mapped_range().context("mapped")?;
    let mut texels = Vec::with_capacity((size * size) as usize);
    for line in data.chunks_exact(padded as usize) {
        let line = &line[..row as usize];
        match format {
            wgpu::TextureFormat::Rgba16Float => {
                texels.extend(line.as_chunks::<8>().0.iter().map(|t| {
                    let h = |i: usize| f16(u16::from_le_bytes([t[i], t[i + 1]]));
                    [h(0), h(2), h(4), h(6)]
                }));
            }
            wgpu::TextureFormat::Rgba8Unorm => {
                texels.extend(
                    line.as_chunks::<4>()
                        .0
                        .iter()
                        .map(|t| std::array::from_fn(|i| f32::from(t[i]) / 255.0)),
                );
            }
            other => bail!("cannot read {other:?}"),
        }
    }
    Ok(Image { size, texels })
}

fn f16(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exp = i32::from((bits >> 10) & 0x1f);
    let frac = f32::from(bits & 0x3ff);
    match exp {
        0 => sign * frac * 2f32.powi(-24),
        31 if frac == 0.0 => sign * f32::INFINITY,
        31 => f32::NAN,
        _ => sign * (1.0 + frac / 1024.0) * 2f32.powi(exp - 15),
    }
}

/// The capture's raw readback of `pass`, if it was kept.
fn reference_pass(dir: &Path, pass: &str) -> Result<Option<Image>> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(None);
    };
    for entry in entries {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        if name.starts_with(pass) && name.ends_with(".rgba16f") {
            let bytes = std::fs::read(&path)?;
            let size = ((bytes.len() / 8) as f64).sqrt() as u32;
            let texels = bytes
                .as_chunks::<8>()
                .0
                .iter()
                .map(|t| {
                    let h = |i: usize| f16(u16::from_le_bytes([t[i], t[i + 1]]));
                    [h(0), h(2), h(4), h(6)]
                })
                .collect();
            return Ok(Some(Image { size, texels }));
        }
    }
    Ok(None)
}

fn load_png(path: &Path) -> Result<Image> {
    let png = image::open(path)
        .with_context(|| path.display().to_string())?
        .to_rgba8();
    Ok(Image {
        size: png.width(),
        texels: png
            .pixels()
            .map(|p| p.0.map(|v| f32::from(v) / 255.0))
            .collect(),
    })
}

/// Premultiplied colour over the page, quantized as the capture's PNGs are.
fn over_bg(image: &Image) -> Image {
    let texels = image
        .texels
        .iter()
        .map(|&[r, g, b, a]| {
            let a = a.clamp(0.0, 1.0);
            let q = |c: f32, bg: f32| {
                ((c + bg * (1.0 - a)).clamp(0.0, 1.0) * 255.0 + 0.5).floor() / 255.0
            };
            [q(r, BG[0]), q(g, BG[1]), q(b, BG[2]), 1.0]
        })
        .collect();
    Image {
        size: image.size,
        texels,
    }
}

fn report(label: &str, theirs: &Image, ours: &Image) {
    if theirs.size != ours.size {
        println!("{label}: size {} vs {}", theirs.size, ours.size);
        return;
    }
    let (mut max, mut sum, mut over) = (0.0_f32, 0.0_f64, 0_usize);
    for (a, b) in theirs.texels.iter().zip(&ours.texels) {
        let d = (0..4).map(|i| (a[i] - b[i]).abs()).fold(0.0, f32::max);
        let d = if d.is_nan() { 1.0 } else { d };
        max = max.max(d);
        sum += f64::from((0..4).map(|i| (a[i] - b[i]).abs()).sum::<f32>()) / 4.0;
        over += usize::from(d > 2.0 / 255.0);
    }
    let n = theirs.texels.len();
    println!(
        "{label:40} max {:8.2}/255  mean {:7.4}/255  >2/255 on {:6.3}% of pixels",
        max * 255.0,
        sum / n as f64 * 255.0,
        over as f64 / n as f64 * 100.0
    );
}

/// Reference, ours, and the difference x8, side by side; small images are
/// scaled up so they can be looked at.
fn side_by_side(theirs: &Image, ours: &Image, path: &Path) -> Result<()> {
    let size = theirs.size.min(ours.size);
    let zoom = (512 / size).max(1);
    let edge = size * zoom;
    let mut out = RgbaImage::new(edge * 3, edge);
    let px = |c: [f32; 4]| Rgba(c.map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8));
    for y in 0..edge {
        for x in 0..edge {
            let i = ((y / zoom) * size + x / zoom) as usize;
            let (a, b) = (theirs.texels[i], ours.texels[i]);
            let opaque = |c: [f32; 4]| [c[0], c[1], c[2], 1.0];
            let d: [f32; 4] = std::array::from_fn(|k| {
                if k == 3 {
                    1.0
                } else {
                    (a[k] - b[k]).abs() * 8.0
                }
            });
            out.put_pixel(x, y, px(opaque(a)));
            out.put_pixel(edge + x, y, px(opaque(b)));
            out.put_pixel(2 * edge + x, y, px(d));
        }
    }
    out.save(path)?;
    Ok(())
}
