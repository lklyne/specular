//! Scripted input for a run nobody is sitting at: an agent checking the
//! window, a scenario replayed in it, or a capture for an ADR.
//!
//! `SPECULAR_SHELL_SCRIPT_FILE` names a scenario script, read by
//! `specular_shell::script` exactly as the headless runner reads it (see
//! "Scenario scripts" in `native/CLAUDE.md`), so a file under
//! `fixtures/scenarios/` runs here as written, `expect` steps included. The
//! run quits when the file ends. `SPECULAR_SHELL_SCRIPT` holds steps
//! separated by `;` and stays open until a `quit` step; there a `key`
//! followed by a number is `keycode`.
//!
//! Pointer and key steps are posted to the application's own event queue,
//! so they take the path a hardware event takes from `sendEvent:` on. Steps
//! that are an app `Action` go through `update`. Positions are in points
//! from the content's top-left corner, which is the app's screen. A step
//! that fails, or a fact that does not hold, ends the run with its text and
//! exit status 1. The clipboard of a scripted run is kept in memory, so it
//! never reads or replaces what the person at the machine copied.
//!
//! ```text
//! wait 1500; control tool.shape; control shape.color.4; shot /tmp/a.png; quit
//! ```
//!
//! The steps only a window can do (`keycode`, `pan`, `swipe`, `resize`,
//! `resize-drag`, `drop`, `menu-choose` and the rest) are listed with
//! `specular_shell::script::WindowStep`. `state PATH` writes the `expect`
//! facts and then the window's own: the Kit's controls on screen and the
//! window's frame.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "each block builds one NSEvent and posts it, or reads one AppKit object"
)]

mod access;
mod control;
mod events;
mod menu;
mod run;
mod steps;
mod us_keys;
mod window;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui_kit::{AnyWindowHandle, App};
use specular_interact::Event;
use specular_shell::script::{Checks, Script};

use self::steps::Do;
use crate::canvas;
use crate::shell;

/// How long the window is given after a step: the events it posted are
/// handled, the models are read again (ten times a second at most) and GPUI
/// lays the next frame out.
const SETTLE: Duration = Duration::from_millis(160);

/// Set when a step failed, so the run ends with a failing status.
static FAILED: AtomicBool = AtomicBool::new(false);

/// The window's number and its content height in points.
fn window_facts() -> Option<(isize, f64)> {
    canvas::with(|canvas| {
        let native = canvas.runtime.window()?.native();
        Some((native.window_number(), native.content_size().1))
    })
    .flatten()
}

/// The content's size in points.
fn content_size() -> Option<(f64, f64)> {
    canvas::with(|canvas| Some(canvas.runtime.window()?.native().content_size())).flatten()
}

/// Whether the environment names a script for this run.
pub(crate) fn scripted() -> bool {
    ["SPECULAR_SHELL_SCRIPT_FILE", "SPECULAR_SHELL_SCRIPT"]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
}

/// A script the environment names, read and lowered.
struct EnvScript {
    /// Each step as written, and what the window does for it.
    steps: Vec<(String, Do)>,
    /// The starting document the `given` lines describe, if any.
    given: Option<specular_doc::Document>,
    /// The folder a `same-as` path starts from.
    base: PathBuf,
    /// Whether the run ends with the script: a file does, inline steps
    /// stay open until a `quit`.
    ends: bool,
}

/// The script the environment names, if it names one.
fn script_from_env() -> Option<Result<EnvScript, String>> {
    let (text, base, ends) = if let Some(path) = std::env::var_os("SPECULAR_SHELL_SCRIPT_FILE") {
        let path = PathBuf::from(path);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => return Some(Err(format!("reading {}: {error}", path.display()))),
        };
        let base = path.parent().map_or_else(PathBuf::new, Path::to_owned);
        (text, base, true)
    } else {
        let script = std::env::var("SPECULAR_SHELL_SCRIPT").ok()?;
        (steps::inline_lines(&script), PathBuf::new(), false)
    };
    Some(read_script(&text, base, ends))
}

fn read_script(text: &str, base: PathBuf, ends: bool) -> Result<EnvScript, String> {
    let script = Script::parse(text).map_err(|error| format!("script {error:#}"))?;
    let given = script
        .given_document()
        .map_err(|error| format!("{error:#}"))?;
    let steps = (script.steps.into_iter())
        .map(|line| match steps::lower(line.step) {
            Ok(step) => Ok((line.text, step)),
            Err(error) => Err(format!(
                "script line {} `{}`: {error}",
                line.number, line.text
            )),
        })
        .collect::<Result<_, _>>()?;
    Ok(EnvScript {
        steps,
        given,
        base,
        ends,
    })
}

/// Reports a failed run and makes its exit status say so.
fn fail(what: &str) {
    eprintln!("specular: {what}");
    FAILED.store(true, Ordering::Relaxed);
}

/// Runs the script the environment names, if there is one.
pub(crate) fn run_from_env(window: AnyWindowHandle, cx: &mut App) {
    let Some(script) = script_from_env() else {
        return;
    };
    // The whole script is read before any of it runs, as headless.
    cx.on_app_quit(|_| {
        if FAILED.load(Ordering::Relaxed) {
            std::process::exit(1);
        }
        async {}
    })
    .detach();
    canvas::with(|canvas| canvas.runtime.script_clipboard(None));
    cx.spawn(async move |cx| {
        let script = match script {
            Ok(script) => script,
            Err(error) => {
                fail(&error);
                cx.update(shell::begin_exit);
                return;
            }
        };
        window::accept_posted_moves();
        // The camera lands at once unless the script turns the spring on,
        // as headless.
        canvas::dispatch(Event::Motion(false));
        if let Some(document) = script.given {
            canvas::dispatch(Event::DocumentOpened(Box::new(document)));
        }
        let begun = canvas::with(|canvas| Checks::begin(canvas.runtime.app(), &script.base));
        let mut checks = match begun {
            Some(Ok(checks)) => checks,
            Some(Err(error)) => {
                fail(&format!("{error:#}"));
                cx.update(shell::begin_exit);
                return;
            }
            None => Checks::default(),
        };
        for (at, (text, step)) in script.steps.into_iter().enumerate() {
            let quits = step == Do::Quit;
            let waits = matches!(step, Do::Wait(_));
            // A check, a mark or a file written posts nothing to wait for.
            let quiet = waits
                || matches!(
                    step,
                    Do::Expect(_) | Do::Mark(_) | Do::State(_) | Do::Save(_)
                );
            if !waits {
                specular_core::ledger::record(specular_core::ledger::Entry::Step(at as u32));
            }
            if let Err(error) = run::step(step, &mut checks, window, cx).await {
                fail(&format!("script step `{text}`: {error}"));
                specular_core::ledger::flush();
                cx.update(shell::begin_exit);
                return;
            }
            if quits {
                return;
            }
            if !quiet {
                cx.background_executor().timer(SETTLE).await;
            }
        }
        specular_core::ledger::flush();
        if script.ends {
            cx.update(shell::begin_exit);
        }
    })
    .detach();
}
