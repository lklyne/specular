//! Command-line arguments.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context as _, bail};
use specular_bench::{GestureProfile, ProfileId, select_profiles};

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
  -h, --help          print this help";

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
}

/// Parses arguments (without the program name).
pub(crate) fn parse(args: impl IntoIterator<Item = OsString>) -> anyhow::Result<Command> {
    let mut run = RunArgs {
        canvas: None,
        pages: None,
        source: SourceKind::default_for_build(),
        bench: None,
        warmup: DEFAULT_WARMUP,
    };
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
            _ if flag.starts_with('-') => bail!("unknown option `{flag}`"),
            _ => set_canvas(&mut run, arg)?,
        }
    }
    if run.canvas.is_some() && run.pages.is_some() {
        bail!("--pages lays out demo pages and cannot be combined with a .canvas file");
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
