//! Command-line arguments.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, bail};
use specular_bench::{GestureProfile, PaintPolicy, ProfileId, select_profiles};

use crate::headless::{self, HeadlessArgs};
use crate::space::SpaceChoice;

/// Usage text for `--help` and argument errors.
pub(crate) const USAGE: &str = "\
usage: specular-app [OPTIONS] [FOLDER | FILE.canvas]

  FOLDER              open the folder as the space: every .canvas file in it
                      is a canvas, and a folder with none gets the starter
                      space
  FILE.canvas         open the folder the file is in as the space, showing
                      that file
                      With neither, and no --space, `specular` opens the
                      space you chose in it, and asks on a first run or when
                      that folder is gone; `specular-app` opens the scratch
                      space. A --bench, --snapshot or --script run shows
                      FILE alone and writes nothing
  --space user|scratch|PATH
                      user: the space the Electron app has open (`spacePath`
                      in its preferences.json), else the one chosen in this
                      app; it is autosaved into. scratch: a copy of the
                      starter space in this app's data folder, so nothing of
                      yours is written to. PATH: the same as a bare FOLDER
                      or FILE.canvas
  --pages N           lay out N demo pages (default 9; not with FILE)
  --source KIND       page backend: synthetic | cef (default: cef when built
                      with the `cef` feature, else synthetic)
  --bench PROFILE     run a scripted pan/zoom profile, print one JSON line per
                      profile to stdout, then exit. PROFILE is `all` or a
                      comma-separated list of:
                      slow-pan, slow-zoom, fast-diagonal-pan, slow-pan-zoom,
                      fast-pan-zoom, zoom-out-then-pan, idle
  --bench-target T    window (default): frames presented in a window, at the
                      display's refresh; headless: frames drawn into a
                      texture with no window, each timed until the GPU is
                      done, sized by --snapshot-size and --snapshot-scale.
                      Either way a line's `work` is each step's time a frame
  --bench-duration-ms N
                      run every profile for N ms in place of its own length.
                      `idle` is a seventh profile, run only when named
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
  --snapshot OUT.png  draw the canvas into a PNG with no window, once its
                      images and Documents have loaded, then exit. Nothing
                      else is written. Pages are synthetic unless `--source
                      cef` is given, which loads the real ones first
  --snapshot-size WxH       the snapshot's viewport in logical pixels
                            (default 1600x1000)
  --snapshot-scale N        device pixels per logical pixel (default 1)
  --snapshot-camera x,y,zoom | fit
                            pan and zoom, or fit the document (the default)
  --theme light|dark|system the snapshot's theme (default: the system's, which
                            a run with no window takes as light)
  --script FILE       with or without --snapshot: scripted input run first,
                      one step a line: click, double-click, move, press,
                      drag-to (x y), release, drag x1 y1 x2 y2, hold MODS,
                      key CHORD, type TEXT, tool NAME, select ID.., camera,
                      wait MS, snapshot OUT.png
  --script-panels on | off
                      whether the built-in toolbar, popup and sidebar are
                      laid out, drawn and clicked (default on). Off, a
                      `control NAME` step does what the control does from
                      the models alone, and no PNG shows a panel
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

/// A space `--space` names by a word, not a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NamedSpace {
    /// `user`.
    User,
    /// `scratch`.
    Scratch,
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
    /// The space `--space user` or `--space scratch` named.
    pub(crate) named_space: Option<NamedSpace>,
    /// Demo page count when no file is given.
    pub(crate) pages: Option<usize>,
    /// Page backend.
    pub(crate) source: SourceKind,
    /// Profiles to run, in order; `None` for an interactive session.
    pub(crate) bench: Option<Vec<GestureProfile>>,
    /// Whether a benchmark draws into a texture and not a window.
    pub(crate) bench_headless: bool,
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
        named_space: None,
        pages: None,
        source: SourceKind::default_for_build(),
        bench: None,
        bench_headless: false,
        warmup: DEFAULT_WARMUP,
        paint_policy: PaintPolicy::ElectronLod,
        window: None,
        chrome: true,
        annotations: 0,
        headless: HeadlessArgs::default(),
    };
    let mut annotations_given = false;
    let mut bench = BenchFlags::default();
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
                run.headless.source = run.source;
            }
            "--space" => match args.next() {
                Some(value) if value == "user" => run.named_space = Some(NamedSpace::User),
                Some(value) if value == "scratch" => run.named_space = Some(NamedSpace::Scratch),
                Some(path) => set_canvas(&mut run, path)?,
                None => {
                    bail!("--space needs a value: user, scratch, or a folder or .canvas file")
                }
            },
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
            _ if snapshot_flag(flag, &mut args, &mut run.headless)? => {}
            _ if bench.take(flag, &mut args)? => {}
            _ if flag.starts_with('-') => bail!("unknown option `{flag}`"),
            _ => set_canvas(&mut run, arg)?,
        }
    }
    run.bench_headless = bench.headless;
    run.bench = bench.profiles()?;
    if run.canvas.is_some() && run.named_space.is_some() {
        bail!("--space user and --space scratch cannot be combined with a path");
    }
    if run.canvas.is_some() && run.pages.is_some() {
        bail!("--pages lays out demo pages and cannot be combined with a .canvas file");
    }
    if annotations_given && !run.chrome {
        bail!("--annotations needs the chrome layer; drop --chrome off");
    }
    Ok(Command::Run(run))
}

/// Reads `flag`'s value into `headless` if it is one of the snapshot
/// flags; `false` if it is not.
fn snapshot_flag(
    flag: &str,
    args: &mut impl Iterator<Item = OsString>,
    headless: &mut HeadlessArgs,
) -> anyhow::Result<bool> {
    match flag {
        "--snapshot" => headless.snapshot = Some(path_of(flag, args.next())?),
        "--script" => headless.script = Some(path_of(flag, args.next())?),
        "--script-panels" => {
            headless.panels = match value_of(flag, args.next())?.as_str() {
                "on" => true,
                "off" => false,
                other => bail!("--script-panels expects on or off, got `{other}`"),
            };
        }
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
struct BenchFlags {
    selection: Option<String>,
    duration: Option<Duration>,
    headless: bool,
}

impl BenchFlags {
    /// Reads `flag`'s value if it is one of these; `false` if it is not.
    fn take(
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
    fn profiles(&self) -> anyhow::Result<Option<Vec<GestureProfile>>> {
        match &self.selection {
            Some(selection) => Ok(Some(profiles_for(selection, self.duration)?)),
            None if self.headless || self.duration.is_some() => {
                bail!("--bench-target and --bench-duration-ms need --bench")
            }
            None => Ok(None),
        }
    }
}

fn set_canvas(run: &mut RunArgs, path: OsString) -> anyhow::Result<()> {
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

impl RunArgs {
    /// Which space the arguments ask for, with `unnamed` for a launch that
    /// names none.
    pub(crate) fn space_choice(&self, unnamed: SpaceChoice) -> SpaceChoice {
        match (&self.canvas, self.named_space) {
            (Some(path), _) => SpaceChoice::Path(path.clone()),
            (None, Some(NamedSpace::User)) => SpaceChoice::User,
            (None, Some(NamedSpace::Scratch)) => SpaceChoice::Scratch,
            (None, None) => unnamed,
        }
    }
}

#[cfg(test)]
mod tests;
