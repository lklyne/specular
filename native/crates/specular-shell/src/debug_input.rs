//! Scripted input for a run nobody is sitting at: an agent checking the
//! window, a scenario replayed in it, or a capture for an ADR.
//!
//! `SPECULAR_SHELL_SCRIPT_FILE` names a script file, one step a line with
//! `#` comments, in the headless runner's vocabulary (`--script`, see
//! `native/CLAUDE.md`), so a file under `fixtures/scenarios/` runs here as
//! written. The run quits when the file ends. `SPECULAR_SHELL_SCRIPT` holds
//! steps separated by `;` and stays open until a `quit` step.
//!
//! Pointer and key steps are posted to the application's own event queue,
//! so they take the path a hardware event takes from `sendEvent:` on. Steps
//! that are an app `Action` go through `update`. Positions are in points
//! from the content's top-left corner, which is the app's screen. A step
//! that fails ends the run with its text and exit status 1; one with no
//! equivalent in a window fails by name. The clipboard of a scripted run is
//! kept in memory, so it never reads or replaces what the person at the
//! machine copied.
//!
//! ```text
//! wait 1500; control tool.shape; control shape.color.4; shot /tmp/a.png; quit
//! ```
//!
//! Besides the headless steps, both forms take:
//!
//! | Step | What it does |
//! |---|---|
//! | `release X Y` | the button comes up at a point |
//! | `scroll X Y DX DY` | a pixel scroll over a point (`wheel` scrolls where the pointer is) |
//! | `keycode CODE [CHARS]`, `cmd-key CODE [CHARS]`, `shift-key CODE [CHARS]` | a key by its `kVK_*` code; Return, Tab, Space, Delete, Escape and the arrows carry their own characters |
//! | `paste-image PATH` | the right panel's composer takes the image file as a paste would hand it over; the system pasteboard is not touched |
//! | `keys shift+cmd CODE [CHARS]` | the same with any modifiers |
//! | `drop PATH.. X Y`, `drop PATH.. nowhere` | files dropped at a point, or off the canvas |
//! | `resize W H` | the window's content resized, its top-left corner kept |
//! | `full-screen` | into full screen, or back out |
//! | `choose NAME` | the first-run choice of that control name, run as its button runs it: for a run with the screen locked, when nothing is drawn to click on |
//! | `menu-dump PATH` | writes the menu bar as `AppKit` shows it |
//! | `menu-choose Menu > Item` | chooses a menu item as a click does |
//! | `state PATH` | writes the selection, the camera, every entity's rect and the window |
//! | `accessibility PATH` | writes what the window exposes to assistive apps |
//! | `shot PATH` | the window as the window server composites it (`snapshot` is the same) |
//! | `quit` | ends the run |
//!
//! In `SPECULAR_SHELL_SCRIPT` a `key` followed by a number is `keycode`,
//! as it was before `key cmd+z` existed here.
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

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gpui_kit::{AnyWindowHandle, App};

use self::steps::{Dialect, Do};
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

/// The script the environment names: its steps as written, and whether the
/// run ends with it.
fn script_from_env() -> Option<Result<(Vec<String>, Dialect), String>> {
    if let Some(path) = std::env::var_os("SPECULAR_SHELL_SCRIPT_FILE") {
        let read = std::fs::read_to_string(&path)
            .map_err(|error| format!("reading {}: {error}", path.display()));
        return Some(read.map(|text| {
            let lines = (text.lines().map(str::trim))
                .filter(|line| !line.is_empty() && !line.starts_with('#'));
            (lines.map(str::to_owned).collect(), Dialect::File)
        }));
    }
    let script = std::env::var("SPECULAR_SHELL_SCRIPT").ok()?;
    let steps = (script.split(';').map(str::trim)).filter(|step| !step.is_empty());
    Some(Ok((steps.map(str::to_owned).collect(), Dialect::Inline)))
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
    let steps = script.and_then(|(lines, dialect)| {
        let parsed: Result<Vec<(String, Do)>, String> = (lines.into_iter())
            .map(|line| match steps::parse(&line, dialect) {
                Ok(step) => Ok((line, step)),
                Err(error) => Err(format!("script step `{line}`: {error}")),
            })
            .collect();
        parsed.map(|steps| (steps, dialect))
    });
    cx.on_app_quit(|_| {
        if FAILED.load(Ordering::Relaxed) {
            std::process::exit(1);
        }
        async {}
    })
    .detach();
    canvas::with(|canvas| canvas.runtime.script_clipboard(None));
    cx.spawn(async move |cx| {
        let (steps, dialect) = match steps {
            Ok(script) => script,
            Err(error) => {
                fail(&error);
                cx.update(shell::begin_exit);
                return;
            }
        };
        for (text, step) in steps {
            let quits = step == Do::Quit;
            let waits = matches!(step, Do::Wait(_));
            if let Err(error) = run::step(step, window, cx).await {
                fail(&format!("script step `{text}`: {error}"));
                cx.update(shell::begin_exit);
                return;
            }
            if quits {
                return;
            }
            if !waits {
                cx.background_executor().timer(SETTLE).await;
            }
        }
        if dialect == Dialect::File {
            cx.update(shell::begin_exit);
        }
    })
    .detach();
}
