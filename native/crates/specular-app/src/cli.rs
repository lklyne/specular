//! Command-line arguments.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, bail};
use specular_bench::{GestureProfile, PaintPolicy, ProfileId, select_profiles};

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
                      shapes at all. With --bench and chrome on, the first
                      page starts selected
  --annotations N     seed N page-bound comment annotations, spread over the
                      pages (needs --chrome on)
  -h, --help          print this help

keys (chrome on):
  Alt + drag a page   move it
  drag a corner       resize the selected page (page re-lays-out on release)
  C                   toggle the comment tool, unless a page has keyboard
                      focus; drag on the canvas to draw an annotation
  Escape              cancel the drag, leave the comment tool, clear page focus";

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

fn value_of(flag: &str, value: Option<OsString>) -> anyhow::Result<String> {
    value
        .with_context(|| format!("{flag} needs a value"))?
        .into_string()
        .map_err(|value| anyhow::anyhow!("{flag} value is not UTF-8: {}", value.display()))
}

/// A `WxH` window size in logical pixels, both sides at least 1.
fn window_size(value: &str) -> anyhow::Result<(u32, u32)> {
    let parsed = value
        .split_once('x')
        .and_then(|(width, height)| Some((width.parse().ok()?, height.parse().ok()?)));
    match parsed {
        Some((width, height)) if width > 0 && height > 0 => Ok((width, height)),
        _ => bail!("--window expects WIDTHxHEIGHT, e.g. 1600x1000, got `{value}`"),
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
mod tests {
    use specular_bench::PROFILES;

    use super::*;

    fn parse_strs(args: &[&str]) -> anyhow::Result<Command> {
        parse(args.iter().map(OsString::from))
    }

    fn run_args(args: &[&str]) -> RunArgs {
        match parse_strs(args).unwrap() {
            Command::Run(run) => run,
            Command::Help => panic!("expected a run"),
        }
    }

    #[test]
    fn no_arguments_runs_default_demo() {
        let run = run_args(&[]);
        assert_eq!((run.canvas, run.pages, run.bench), (None, None, None));
    }

    #[test]
    fn pages_flag_sets_demo_page_count() {
        assert_eq!(run_args(&["--pages", "20"]).pages, Some(20));
    }

    #[test]
    fn zero_pages_is_rejected() {
        assert!(parse_strs(&["--pages", "0"]).is_err());
    }

    #[test]
    fn source_flag_selects_synthetic() {
        assert_eq!(
            run_args(&["--source", "synthetic"]).source,
            SourceKind::Synthetic
        );
    }

    #[test]
    fn unknown_source_is_rejected() {
        assert!(parse_strs(&["--source", "webkit"]).is_err());
    }

    #[test]
    fn bench_all_selects_every_profile_in_order() {
        let ids: Vec<_> = run_args(&["--bench", "all"])
            .bench
            .unwrap()
            .iter()
            .map(|profile| profile.id)
            .collect();
        assert_eq!(ids, PROFILES.map(|profile| profile.id));
    }

    #[test]
    fn bench_accepts_electron_profile_id() {
        let bench = run_args(&["--bench", "fast-pan-zoom"]).bench.unwrap();
        assert_eq!(bench[0].id, ProfileId::FastPanZoom);
    }

    #[test]
    fn bench_list_runs_in_electron_order() {
        let ids: Vec<_> = run_args(&["--bench", "slow-zoom,slow-pan"])
            .bench
            .unwrap()
            .iter()
            .map(|profile| profile.id)
            .collect();
        assert_eq!(ids, [ProfileId::SlowPan, ProfileId::SlowZoom]);
    }

    #[test]
    fn warmup_ms_sets_bench_warmup() {
        let run = run_args(&["--bench", "all", "--warmup-ms", "8000"]);
        assert_eq!(run.warmup, Duration::from_secs(8));
    }

    #[test]
    fn window_flag_sets_logical_size() {
        assert_eq!(
            run_args(&["--window", "1600x1000"]).window,
            Some((1600, 1000))
        );
    }

    #[test]
    fn window_flag_rejects_a_zero_side() {
        assert!(parse_strs(&["--window", "0x600"]).is_err());
    }

    #[test]
    fn chrome_defaults_on_without_annotations() {
        let run = run_args(&[]);
        assert_eq!((run.chrome, run.annotations), (true, 0));
    }

    #[test]
    fn chrome_flag_turns_the_layer_off() {
        assert!(!run_args(&["--chrome", "off"]).chrome);
    }

    #[test]
    fn unknown_chrome_value_is_rejected() {
        assert!(parse_strs(&["--chrome", "maybe"]).is_err());
    }

    #[test]
    fn annotations_flag_sets_the_seed_count() {
        assert_eq!(run_args(&["--annotations", "40"]).annotations, 40);
    }

    #[test]
    fn annotations_with_chrome_off_is_rejected() {
        assert!(parse_strs(&["--chrome", "off", "--annotations", "5"]).is_err());
        assert!(parse_strs(&["--annotations", "5", "--chrome", "off"]).is_err());
    }

    #[test]
    fn annotations_must_be_a_number() {
        assert!(parse_strs(&["--annotations", "lots"]).is_err());
    }

    #[test]
    fn paint_policy_defaults_to_electron_lod() {
        assert_eq!(run_args(&[]).paint_policy, PaintPolicy::ElectronLod);
    }

    #[test]
    fn paint_policy_flag_selects_full_rate() {
        assert_eq!(
            run_args(&["--paint-policy", "full-rate"]).paint_policy,
            PaintPolicy::FullRate
        );
    }

    #[test]
    fn unknown_paint_policy_is_rejected() {
        assert!(parse_strs(&["--paint-policy", "fast"]).is_err());
    }

    #[test]
    fn only_cef_frames_are_representative() {
        assert_eq!(
            [SourceKind::Cef, SourceKind::Synthetic].map(SourceKind::is_representative),
            [true, false]
        );
    }

    #[test]
    fn unknown_bench_profile_is_rejected() {
        assert!(parse_strs(&["--bench", "spin"]).is_err());
    }

    #[test]
    fn positional_argument_is_canvas_path() {
        assert_eq!(
            run_args(&["demo.canvas"]).canvas,
            Some(PathBuf::from("demo.canvas"))
        );
    }

    #[test]
    fn pages_with_canvas_file_is_rejected() {
        assert!(parse_strs(&["demo.canvas", "--pages", "3"]).is_err());
    }

    #[test]
    fn flag_missing_value_is_rejected() {
        assert!(parse_strs(&["--pages"]).is_err());
    }

    #[test]
    fn help_flag_wins_over_other_arguments() {
        assert_eq!(parse_strs(&["--pages", "3", "-h"]).unwrap(), Command::Help);
    }
}
