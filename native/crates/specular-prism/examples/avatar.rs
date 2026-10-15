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
use common::{gpu, over, render};
use image::{Rgba, RgbaImage, imageops};
use specular_prism::{Alpha, PrismOptions, PrismRenderer};

mod common;

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
