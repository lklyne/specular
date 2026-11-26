//! `specular-mod-check <dir> [--state fixture.json]`: what `specular mod
//! check` runs.
//!
//! It loads the mod in `<dir>` with no window and no app, renders it once
//! against a state fixture (a JSON object of key to value, as `GET /state`
//! answers, or a fresh app's state when none is given), and prints the
//! element tree it described with ids, the commands it ran and its
//! `console` lines. A mod that fails exits 1 and gets `<name>.error` beside
//! its folder; one that passes has it taken away.
//!
//! GPUI's test platform is the window: it lays out and draws nothing, on
//! any machine, with or without a display.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, bail};
use gpui::{TestAppContext, TestDispatcher, VisualTestContext};
use serde_json::{Map, Value};
use specular_mods::{check, fresh_state, shipped};

const USAGE: &str = "usage: specular mod check <dir> [--state fixture.json]";

struct Args {
    dir: PathBuf,
    state: Option<PathBuf>,
}

fn parse(mut args: impl Iterator<Item = String>) -> anyhow::Result<Args> {
    let mut dir = None;
    let mut state = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--state" => state = Some(PathBuf::from(args.next().context(USAGE)?)),
            "-h" | "--help" => bail!(USAGE),
            _ if dir.is_none() => dir = Some(PathBuf::from(arg)),
            _ => bail!("unexpected `{arg}`\n{USAGE}"),
        }
    }
    Ok(Args {
        dir: dir.context(USAGE)?,
        state,
    })
}

fn fixture(path: Option<&Path>) -> anyhow::Result<Map<String, Value>> {
    let Some(path) = path else {
        return Ok(fresh_state());
    };
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    match serde_json::from_str(&text).with_context(|| format!("reading {}", path.display()))? {
        Value::Object(keys) => Ok(keys),
        _ => bail!(
            "{} is not a JSON object of state keys (`specular state > fixture.json` writes one)",
            path.display()
        ),
    }
}

/// A mod's `console` lines, printed as they come.
fn print_console() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(
            "gpui_shell::script=debug",
        ))
        .without_time()
        .with_target(false)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .with_writer(std::io::stdout)
        .try_init();
}

fn run(args: &Args) -> anyhow::Result<()> {
    let fixture = fixture(args.state.as_deref())?;
    let data = std::env::temp_dir().join(format!("specular-mod-check-{}", std::process::id()));
    let mut cx = TestAppContext::build(TestDispatcher::new(0), None);
    let window = cx.add_window(|_, _| gpui::Empty);
    let mut vcx = VisualTestContext::from_window(*window, &cx);
    let outcome = vcx.update(|window, cx| check(&args.dir, &data, &fixture, window, cx));
    let _ = std::fs::remove_dir_all(&data);
    let report = match outcome {
        Ok(report) => report,
        Err(error) => {
            let message = format!("{error:#}");
            if let Err(write) = shipped::write_error(&args.dir, &message) {
                eprintln!("could not write the error file: {write}");
            }
            bail!(message);
        }
    };
    let _ = shipped::clear_error(&args.dir);
    println!("{}", report.tree.trim_end());
    for run in &report.runs {
        println!("ran: {run}");
    }
    for notice in &report.notices {
        println!("notice: {notice}");
    }
    Ok(())
}

fn main() -> ExitCode {
    print_console();
    let outcome = parse(std::env::args().skip(1)).and_then(|args| run(&args));
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}
