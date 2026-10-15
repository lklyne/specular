//! The tumble as frames: the Prism posed by [`Tumble`] at 60 Hz, working
//! for `WORK` seconds and then settling, for judging the motion and the
//! framing by eye.
//!
//! ```sh
//! cargo run -p specular-prism --example tumble -- OUT_DIR [SIZE] [HALF_EXTENT] [SEED]
//! ```
//!
//! Writes `frames/NNNN.png` (straight alpha, over nothing) for a movie,
//! `sheet.png` (every third frame over the pill's light and dark fills,
//! nearest-scaled 3x) and prints how many frames the shape touches the
//! edge in, which is where it would be clipped.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, Result};
use common::{gpu, over, render};
use image::{Rgba, RgbaImage, imageops};
use specular_prism::{Alpha, PrismOptions, PrismRenderer, Tumble};

mod common;

/// Seconds of work, then of settling.
const WORK: f32 = 4.0;
const SETTLE: f32 = 1.5;
const RATE: u32 = 60;
/// The pill's fills, light and dark.
const LIGHT: [u8; 3] = [0xfa, 0xfa, 0xf9];
const DARK: [u8; 3] = [0x33, 0x2f, 0x2d];
/// Seconds into the scene of the first frame: the still pose.
const STILL: f32 = 4.0;
const EVERY: usize = 3;
const ZOOM: u32 = 3;
const COLUMNS: u32 = 10;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(
        args.next()
            .context("usage: tumble OUT_DIR [SIZE] [HALF_EXTENT] [SEED]")?,
    );
    let size: u32 = args.next().map_or(Ok(68), |s| s.parse())?;
    let half: f32 = args.next().map_or(Ok(0.5), |s| s.parse())?;
    let seed: u64 = args.next().map_or(Ok(1), |s| s.parse())?;
    let frames_dir = out.join("frames");
    std::fs::create_dir_all(&frames_dir)?;
    let (device, queue) = pollster::block_on(gpu())?;
    let mut prism = PrismRenderer::new(
        &device,
        &queue,
        PrismOptions {
            alpha: Alpha::Straight,
            shell_segments: 24,
            ..PrismOptions::new(size, 3)
        },
    );
    prism.params_mut().camera.half_extent = half;

    let mut tumble = Tumble::new(seed);
    let frame = Duration::from_secs(1) / RATE;
    let count = ((WORK + SETTLE) * RATE as f32) as usize;
    let mut shots = Vec::new();
    let mut touching = 0;
    for index in 0..count {
        let t = index as f32 / RATE as f32;
        prism.params_mut().pose = tumble.pose();
        let shot = render(&device, &queue, &mut prism, STILL + t)?;
        if touches_edge(&shot) {
            touching += 1;
        }
        shot.save(frames_dir.join(format!("{index:04}.png")))?;
        shots.push(shot);
        tumble.advance(frame, t < WORK);
    }
    println!(
        "{count} frames, {touching} touch the edge ({:.0}%), at rest at the end: {}",
        100.0 * touching as f32 / count as f32,
        tumble.at_rest()
    );

    let picked: Vec<&RgbaImage> = shots.iter().step_by(EVERY).collect();
    let cell = size * ZOOM;
    let rows = (picked.len() as u32).div_ceil(COLUMNS);
    let mut sheet = RgbaImage::from_pixel(
        COLUMNS * (cell + 4),
        rows * 2 * (cell + 4),
        Rgba([128, 128, 128, 255]),
    );
    for (i, shot) in picked.iter().enumerate() {
        let (column, row) = (i as u32 % COLUMNS, i as u32 / COLUMNS);
        for (side, bg) in [LIGHT, DARK].into_iter().enumerate() {
            let big = imageops::resize(&over(shot, bg), cell, cell, imageops::FilterType::Nearest);
            let y = (row * 2 + side as u32) * (cell + 4);
            imageops::replace(
                &mut sheet,
                &big,
                i64::from(column * (cell + 4)),
                i64::from(y),
            );
        }
    }
    sheet.save(out.join("sheet.png"))?;
    println!(
        "sheet: every {EVERY}th frame of {RATE} Hz, {COLUMNS} a row, light then dark; work ends at frame {}",
        (WORK * RATE as f32) as usize
    );
    Ok(())
}

/// Whether any edge pixel is more than faintly covered.
fn touches_edge(shot: &RgbaImage) -> bool {
    let (w, h) = shot.dimensions();
    let covered = |x: u32, y: u32| shot.get_pixel(x, y)[3] > 24;
    (0..w).any(|x| covered(x, 0) || covered(x, h - 1))
        || (0..h).any(|y| covered(0, y) || covered(w - 1, y))
}
