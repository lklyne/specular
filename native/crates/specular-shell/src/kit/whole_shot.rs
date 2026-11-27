//! The whole window as a PNG, panels and all, for `specular shot --whole`.
//!
//! The panels are GPUI's and the canvas is ours, each drawn into its own
//! layer, so neither can read the other back. The window server has both:
//! `screencapture -l` asks it for this window alone. macOS asks the person
//! once to let Specular record the screen.

use std::process::Command;

use anyhow::Context as _;

/// The window numbered `window`, as the screen shows it, with its size in
/// device pixels.
pub(crate) fn capture(window: isize) -> anyhow::Result<(Vec<u8>, u32, u32)> {
    let path = std::env::temp_dir().join(format!("specular-whole-{}.png", std::process::id()));
    let status = Command::new("/usr/sbin/screencapture")
        .args(["-x", "-o", "-t", "png"])
        .arg(format!("-l{window}"))
        .arg(&path)
        .status()
        .context("running screencapture")?;
    let png = std::fs::read(&path);
    let _ = std::fs::remove_file(&path);
    anyhow::ensure!(
        status.success(),
        "screencapture failed ({status}): the window must be shown on the current Space, not \
         hidden, minimized or on another one, and Specular needs Screen Recording permission \
         in System Settings > Privacy & Security"
    );
    let png = png.context("screencapture wrote no picture; allow Specular to record the screen in System Settings, Privacy & Security")?;
    let (width, height) = size(&png).context("screencapture wrote something other than a PNG")?;
    Ok((png, width, height))
}

/// A PNG's width and height, from its header.
fn size(png: &[u8]) -> Option<(u32, u32)> {
    let header = png.get(16..24)?;
    let word = |at: usize| Some(u32::from_be_bytes(header.get(at..at + 4)?.try_into().ok()?));
    Some((word(0)?, word(4)?))
}
