//! Command-line arguments.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, bail};
use specular_bench::{GestureProfile, PaintPolicy, ProfileId, select_profiles};

use crate::headless::{self, HeadlessArgs};

/// Usage text for `--help` and argument errors.
pub(crate) const USAGE: &str = "\
usage: specular-app [OPTIONS] [FILE.canvas]

  --pages N           lay out N demo pages (default 9; not with FILE)
  --source KIND       page backend: synthetic | cef (default: cef when built
                      with the `cef` feature, else synthetic)
  --bench PROFILE     run a scripted pan/zoom profile, print one JSON line per
                      profile to stdout, then exit. PROFILE is `all` or a
                      comma-separated list of:
                      slow-pan, slow-zoom, fast-diagonal-pan, slow-pan-zoom,
                      fast-pan-zoom, zoom-out-then-pan
  --warmup-ms N       with --bench: let pages load and settle for N ms before
                      the first profile (default 2000)
  --window WxH        window size in logical pixels, e.g. 1600x1000 (default:
                      the platform's)
  --paint-policy P    electron-lod (default): Electron's page-host frame-rate
                      and texture-scale tiers plus off-screen culling;
                      full-rate: every page paints at full rate and scale
  --chrome on|off     on (default): draw the canvas chrome (page borders,
                      selection with resize handles, comment annotations)
                      through the shape layer every frame; off: draw no
                      shapes at all (the keys below still act, unseen). With
                      --bench and chrome on, the first page starts selected
  --annotations N     seed N page-bound comment annotations, spread over the
                      pages (needs --chrome on)
  --snapshot OUT.png  draw the canvas into a PNG with no window, on the
                      synthetic source, once its images and Documents have
                      loaded, then exit. Nothing else is written
  --snapshot-size WxH       the snapshot's viewport in logical pixels
                            (default 1600x1000)
  --snapshot-scale N        device pixels per logical pixel (default 1)
  --snapshot-camera x,y,zoom | fit
                            pan and zoom, or fit the document (the default)
  --script FILE       with or without --snapshot: scripted input run first,
                      one step a line: click, double-click, move, press,
                      drag-to (x y), release, drag x1 y1 x2 y2, hold MODS,
                      key CHORD, type TEXT, tool NAME, select ID.., camera,
                      wait MS, snapshot OUT.png
  -h, --help          print this help

keys:
  Alt + drag a page   move it
  drag a corner       resize the selected page (page re-lays-out on release)
  C                   the comment tool: click a point or a page, or drag a
                      region, then type the comment and press Return
  Cmd+Z, Cmd+Shift+Z  undo and redo a move, a resize or an annotation
  Escape              drop the comment being written; else cancel the drag,
                      leave the comment tool, clear page focus
                      (C and Cmd+Z go to the page while one has keyboard
                      focus, so press Escape first)";

/// Settle time before the first bench profile when `--warmup-ms` is not given.
const DEFAULT_WARMUP: Duration = Duration::from_secs(2);

/// Which page backend to host pages in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SourceKind {
    /// Animated CPU frames; runs anywhere, never representative.
    Synthetic,
    /// Chromium via CEF offscreen rendering.
    Cef,
}

impl SourceKind {
    /// Whether frames from this source may be compared with Electron's
    /// (ADR 0038): only CEF's; synthetic frames are CPU-painted stand-ins.
    pub(crate) const fn is_representative(self) -> bool {
        matches!(self, Self::Cef)
    }

    fn default_for_build() -> Self {
        if cfg!(feature = "cef") {
            Self::Cef
        } else {
            Self::Synthetic
        }
    }
}

/// What the user asked for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Command {
    /// Print usage and exit.
    Help,
    /// Open the window.
    Run(RunArgs),
}

/// Arguments for a normal or benchmark run.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunArgs {
    /// `.canvas` file to load pages from.
    pub(crate) canvas: Option<PathBuf>,
    /// Demo page count when no file is given.
    pub(crate) pages: Option<usize>,
    /// Page backend.
    pub(crate) source: SourceKind,
    /// Profiles to run, in order; `None` for an interactive session.
    pub(crate) bench: Option<Vec<GestureProfile>>,
    /// Settle time before the first bench profile.
    pub(crate) warmup: Duration,
    /// How pages are throttled.
    pub(crate) paint_policy: PaintPolicy,
    /// Window size in logical pixels; `None` takes the platform default.
    pub(crate) window: Option<(u32, u32)>,
    /// Whether the chrome layer is drawn.
    pub(crate) chrome: bool,
    /// Page-bound annotations to seed at startup.
    pub(crate) annotations: usize,
    /// Snapshots to draw instead of opening a window.
    pub(crate) headless: HeadlessArgs,
}

/// Parses arguments (without the program name).
pub(crate) fn parse(args: impl IntoIterator<Item = OsString>) -> anyhow::Result<Command> {
    let mut run = RunArgs {
        canvas: None,
        pages: None,
        source: SourceKind::default_for_build(),
        bench: None,
        warmup: DEFAULT_WARMUP,
        paint_policy: PaintPolicy::ElectronLod,
        window: None,
        chrome: true,
        annotations: 0,
        headless: HeadlessArgs::default(),
    };
    let mut annotations_given = false;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let Some(flag) = arg.to_str() else {
            set_canvas(&mut run, arg)?;
            continue;
        };
        match flag {
            "-h" | "--help" => return Ok(Command::Help),
            "--pages" => {
                let value = value_of(flag, args.next())?;
                let pages: usize = value
                    .parse()
                    .with_context(|| format!("--pages expects a number, got `{value}`"))?;
                if pages == 0 {
                    bail!("--pages must be at least 1");
                }
                run.pages = Some(pages);
            }
            "--source" => {
                run.source = match value_of(flag, args.next())?.as_str() {
                    "synthetic" => SourceKind::Synthetic,
                    "cef" => SourceKind::Cef,
                    other => bail!("unknown --source `{other}` (expected synthetic or cef)"),
                };
            }
            "--bench" => run.bench = Some(profiles_for(&value_of(flag, args.next())?)?),
            "--warmup-ms" => {
                let value = value_of(flag, args.next())?;
                let ms: u64 = value
                    .parse()
                    .with_context(|| format!("--warmup-ms expects a number, got `{value}`"))?;
                run.warmup = Duration::from_millis(ms);
            }
            "--paint-policy" => {
                run.paint_policy = value_of(flag, args.next())?.parse()?;
            }
            "--chrome" => {
                run.chrome = match value_of(flag, args.next())?.as_str() {
                    "on" => true,
                    "off" => false,
                    other => bail!("unknown --chrome `{other}` (expected on or off)"),
                };
            }
            "--annotations" => {
                let value = value_of(flag, args.next())?;
                run.annotations = value
                    .parse()
                    .with_context(|| format!("--annotations expects a number, got `{value}`"))?;
                annotations_given = true;
            }
            "--window" => run.window = Some(window_size(&value_of(flag, args.next())?)?),
            "--snapshot" => run.headless.snapshot = Some(path_of(flag, args.next())?),
            "--script" => run.headless.script = Some(path_of(flag, args.next())?),
            "--snapshot-size" => {
                run.headless.size = window_size(&value_of(flag, args.next())?)?;
            }
            "--snapshot-scale" => {
                let value = value_of(flag, args.next())?;
                run.headless.scale = match value.parse::<f32>() {
                    Ok(scale) if (0.25..=8.0).contains(&scale) => scale,
                    _ => bail!("--snapshot-scale expects a number from 0.25 to 8, got `{value}`"),
                };
            }
            "--snapshot-camera" => {
                run.headless.camera = headless::camera_arg(&value_of(flag, args.next())?)?;
            }
            _ if flag.starts_with('-') => bail!("unknown option `{flag}`"),
            _ => set_canvas(&mut run, arg)?,
        }
    }
    if run.canvas.is_some() && run.pages.is_some() {
        bail!("--pages lays out demo pages and cannot be combined with a .canvas file");
    }
    if annotations_given && !run.chrome {
        bail!("--annotations needs the chrome layer; drop --chrome off");
    }
    Ok(Command::Run(run))
}

fn set_canvas(run: &mut RunArgs, path: OsString) -> anyhow::Result<()> {
    if run.canvas.is_some() {
        bail!("only one .canvas file may be given");
    }
    run.canvas = Some(PathBuf::from(path));
    Ok(())
}

fn path_of(flag: &str, value: Option<OsString>) -> anyhow::Result<PathBuf> {
    Ok(PathBuf::from(
        value.with_context(|| format!("{flag} needs a value"))?,
    ))
}

fn value_of(flag: &str, value: Option<OsString>) -> anyhow::Result<String> {
    value
        .with_context(|| format!("{flag} needs a value"))?
        .into_string()
        .map_err(|value| anyhow::anyhow!("{flag} value is not UTF-8: {}", value.display()))
}

/// A `WxH` size in logical pixels, both sides at least 1.
fn window_size(value: &str) -> anyhow::Result<(u32, u32)> {
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
fn profiles_for(selection: &str) -> anyhow::Result<Vec<GestureProfile>> {
    if selection == "all" {
        return Ok(select_profiles(&[], None));
    }
    let ids = selection
        .split(',')
        .map(|id| id.trim().parse::<ProfileId>())
        .collect::<Result<Vec<_>, _>>()?;
    Ok(select_profiles(&ids, None))
}

#[cfg(test)]
mod tests;
