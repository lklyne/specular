//! The values the flags take: snapshot flags, bench flags, paths and sizes.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, bail};
use specular_bench::{GestureProfile, ProfileId, select_profiles};

use super::RunArgs;
use crate::headless::{self, HeadlessArgs};

/// Reads `flag`'s value into `headless` if it is one of the snapshot
/// flags; `false` if it is not.
pub(super) fn snapshot_flag(
    flag: &str,
    args: &mut impl Iterator<Item = OsString>,
    headless: &mut HeadlessArgs,
) -> anyhow::Result<bool> {
    match flag {
        "--snapshot" => headless.snapshot = Some(path_of(flag, args.next())?),
        "--script" => headless.script = Some(path_of(flag, args.next())?),
        "--snapshot-size" => {
            headless.size = window_size(&value_of(flag, args.next())?)?;
        }
        "--snapshot-scale" => {
            let value = value_of(flag, args.next())?;
            headless.scale = match value.parse::<f32>() {
                Ok(scale) if (0.25..=8.0).contains(&scale) => scale,
                _ => bail!("--snapshot-scale expects a number from 0.25 to 8, got `{value}`"),
            };
        }
        "--theme" => headless.theme = Some(headless::theme_arg(&value_of(flag, args.next())?)?),
        "--snapshot-camera" => {
            headless.camera = headless::camera_arg(&value_of(flag, args.next())?)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// The benchmark's flags, which only mean something together.
#[derive(Debug, Default)]
pub(super) struct BenchFlags {
    selection: Option<String>,
    duration: Option<Duration>,
    pub(super) headless: bool,
}

impl BenchFlags {
    /// Reads `flag`'s value if it is one of these; `false` if it is not.
    pub(super) fn take(
        &mut self,
        flag: &str,
        args: &mut impl Iterator<Item = OsString>,
    ) -> anyhow::Result<bool> {
        match flag {
            "--bench" => self.selection = Some(value_of(flag, args.next())?),
            "--bench-target" => {
                self.headless = match value_of(flag, args.next())?.as_str() {
                    "window" => false,
                    "headless" => true,
                    other => {
                        bail!("unknown --bench-target `{other}` (expected window or headless)")
                    }
                };
            }
            "--bench-duration-ms" => {
                let value = value_of(flag, args.next())?;
                let ms: u64 = value.parse().with_context(|| {
                    format!("--bench-duration-ms expects a number, got `{value}`")
                })?;
                self.duration = Some(Duration::from_millis(ms));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// The profiles to run, or `None` when no benchmark was asked for.
    pub(super) fn profiles(&self) -> anyhow::Result<Option<Vec<GestureProfile>>> {
        match &self.selection {
            Some(selection) => Ok(Some(profiles_for(selection, self.duration)?)),
            None if self.headless || self.duration.is_some() => {
                bail!("--bench-target and --bench-duration-ms need --bench")
            }
            None => Ok(None),
        }
    }
}

pub(super) fn set_canvas(run: &mut RunArgs, path: OsString) -> anyhow::Result<()> {
    if run.canvas.is_some() {
        bail!("only one folder or .canvas file may be given");
    }
    run.canvas = Some(PathBuf::from(path));
    Ok(())
}

fn path_of(flag: &str, value: Option<OsString>) -> anyhow::Result<PathBuf> {
    Ok(PathBuf::from(
        value.with_context(|| format!("{flag} needs a value"))?,
    ))
}

pub(super) fn value_of(flag: &str, value: Option<OsString>) -> anyhow::Result<String> {
    value
        .with_context(|| format!("{flag} needs a value"))?
        .into_string()
        .map_err(|value| anyhow::anyhow!("{flag} value is not UTF-8: {}", value.display()))
}

/// A `WxH` size in logical pixels, both sides at least 1.
pub(super) fn window_size(value: &str) -> anyhow::Result<(u32, u32)> {
    let parsed = value
        .split_once('x')
        .and_then(|(width, height)| Some((width.parse().ok()?, height.parse().ok()?)));
    match parsed {
        Some((width, height)) if width > 0 && height > 0 => Ok((width, height)),
        _ => bail!("a size is WIDTHxHEIGHT, e.g. 1600x1000, got `{value}`"),
    }
}

/// The profiles named by `selection`: `all`, or comma-separated Electron
/// profile ids (run in Electron's order, as `/perf/pan-zoom/run` does).
fn profiles_for(
    selection: &str,
    duration: Option<Duration>,
) -> anyhow::Result<Vec<GestureProfile>> {
    if selection == "all" {
        return Ok(select_profiles(&[], duration));
    }
    let ids = selection
        .split(',')
        .map(|id| id.trim().parse::<ProfileId>())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(select_profiles(&ids, duration))
}
