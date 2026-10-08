//! Scripted input for a run nobody is sitting at: an agent checking the
//! window, or a capture for an ADR.
//!
//! `SPECULAR_SHELL_SCRIPT` holds steps separated by `;`. Pointer and key
//! steps are posted to the application's own event queue, so they take the
//! path a hardware event takes from `sendEvent:` on. Positions are in points
//! from the content's top-left corner.
//!
//! ```text
//! wait 1500; click 800 22; wait 400; shot /tmp/toolbar.png; quit
//! ```
//!
//! | Step | What it does |
//! |---|---|
//! | `wait MS` | waits |
//! | `move X Y`, `press X Y`, `release X Y` | one pointer event |
//! | `click X Y`, `double-click X Y`, `right-click X Y` | a whole click |
//! | `hold shift+cmd`, `hold none` | modifiers for the pointer steps after it |
//! | `scroll X Y DX DY` | a pixel scroll over a point |
//! | `drag X1 Y1 X2 Y2` | a press, the pointer moved across in steps, a release |
//! | `key CODE [CHARS]`, `cmd-key CODE [CHARS]`, `shift-key CODE [CHARS]` | a key by its `kVK_*` code; Return, Tab, Space, Delete, Escape and the arrows carry their own characters |
//! | `keys shift+cmd CODE [CHARS]` | the same with any modifiers |
//! | `type TEXT` | US-layout keys for each character |
//! | `paste-image PATH` | the composer takes the image file as a paste would hand it over; the system pasteboard is not touched |
//! | `drop PATH X Y` | drops a file at a point, as a drag from Finder ends |
//! | `choose NAME` | the first-run choice of that control name, run as its button runs it: for a run with the screen locked, when nothing is drawn to click on |
//! | `menu-dump PATH` | writes the menu bar as `AppKit` shows it |
//! | `menu-choose Menu > Item` | chooses a menu item as a click does |
//! | `state PATH` | writes the selection, the camera and every entity's rect |
//! | `shot PATH` | the window as the window server composites it |
//! | `quit` | ends the run |
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "each block builds one NSEvent and posts it"
)]

mod events;
mod menu;

use std::cell::Cell;
use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use gpui_kit::{AnyWindowHandle, App};

use self::events::{LEFT_DOWN, LEFT_DRAGGED, LEFT_UP, MOVED, mouse};
use crate::canvas;
use crate::shell;

thread_local! {
    /// The modifier flags `hold` set for the pointer steps after it.
    static HELD: Cell<usize> = const { Cell::new(0) };
}

/// The window's number and its content height in points.
fn window_facts() -> Option<(isize, f64)> {
    canvas::with(|canvas| {
        let native = canvas.runtime.window()?.native();
        Some((native.window_number(), native.content_size().1))
    })
    .flatten()
}

fn shot(path: &str) {
    let Some((number, _)) = window_facts() else {
        return;
    };
    let taken = Command::new("screencapture")
        .args(["-x", "-o", "-l", &number.to_string(), path])
        .status();
    match taken {
        Ok(status) if status.success() => tracing::info!(path, "scripted input: shot"),
        other => tracing::warn!(path, "scripted input: shot failed: {other:?}"),
    }
}

fn write(path: &str, text: &str) {
    match std::fs::write(path, text) {
        Ok(()) => tracing::info!(path, "scripted input: written"),
        Err(error) => tracing::warn!(path, "scripted input: not written: {error}"),
    }
}

/// What a check reads back: the canvas showing, the camera, the selection
/// and every entity's rect, one a line.
fn state() -> String {
    canvas::with(|canvas| {
        let app = canvas.runtime.app();
        let camera = app.session().camera;
        let mut out = String::new();
        let _ = writeln!(out, "canvas {}", app.space().active().name);
        let _ = writeln!(
            out,
            "camera {:.1} {:.1} {:.3}",
            camera.pan.x, camera.pan.y, camera.zoom
        );
        let _ = writeln!(out, "covered-left {}", app.covered_left());
        let _ = writeln!(out, "tool {:?}", app.session().tool);
        let _ = writeln!(out, "selection {:?}", app.session().selection.items());
        for entity in app.document().entities() {
            let rect = entity.rect;
            let _ = writeln!(
                out,
                "entity {} {:.1} {:.1} {:.1} {:.1}",
                entity.id, rect.x, rect.y, rect.width, rect.height
            );
        }
        out
    })
    .unwrap_or_default()
}

/// How many moves a `drag` makes between its ends.
const DRAG_STEPS: u32 = 8;

/// The characters `AppKit` gives a key that types none of its own. GPUI
/// names a key by them, so Return without `\r` is no key at all.
const fn named_characters(code: u16) -> Option<&'static str> {
    match code {
        36 | 76 => Some("\r"),
        48 => Some("\t"),
        49 => Some(" "),
        51 => Some("\u{7f}"),
        53 => Some("\u{1b}"),
        123 => Some("\u{f702}"),
        124 => Some("\u{f703}"),
        125 => Some("\u{f701}"),
        126 => Some("\u{f700}"),
        _ => None,
    }
}

/// Runs the first-run view's choice named `name`.
fn choose(name: &str) {
    let action = canvas::models()
        .and_then(|models| models.onboarding)
        .and_then(|view| view.choices.into_iter().find(|choice| choice.name == name))
        .map(|choice| choice.action);
    if let Some(action) = action {
        canvas::dispatch(specular_interact::Event::Action(action));
    } else {
        tracing::warn!(
            name,
            "scripted input: the first-run view has no such choice"
        );
    }
}

/// What one step does, or how long it waits.
enum Step {
    Wait(Duration),
    /// Run outside any GPUI update: `AppKit` validates the menu bar by
    /// calling back into GPUI, which must not be borrowed then.
    Run(Box<dyn FnOnce()>),
    /// The composer is handed this image file as a paste.
    PasteImage(PathBuf),
    Quit,
}

fn run(step: impl FnOnce() + 'static) -> Step {
    Step::Run(Box::new(step))
}

fn parse(step: &str) -> Option<Step> {
    let (verb, rest) = step.split_once(char::is_whitespace).unwrap_or((step, ""));
    let rest = rest.trim().to_owned();
    let mut words = rest.split_whitespace();
    let mut number = || words.next()?.parse::<f64>().ok();
    let step = match verb {
        "wait" => Step::Wait(Duration::from_millis(number()? as u64)),
        "move" | "press" | "release" | "click" | "double-click" | "right-click" => {
            let (x, y) = (number()?, number()?);
            let verb = verb.to_owned();
            run(move || {
                let held = HELD.get();
                let send = |kind, clicks| mouse(kind, x, y, clicks, held);
                match verb.as_str() {
                    "move" => send(MOVED, 0),
                    "press" => send(LEFT_DOWN, 1),
                    "release" => send(LEFT_UP, 1),
                    "right-click" => {
                        send(MOVED, 0);
                        events::right_click(x, y);
                    }
                    "click" => [MOVED, LEFT_DOWN, LEFT_UP]
                        .into_iter()
                        .for_each(|kind| send(kind, isize::from(kind != MOVED))),
                    _ => {
                        send(MOVED, 0);
                        for clicks in [1, 2] {
                            send(LEFT_DOWN, clicks);
                            send(LEFT_UP, clicks);
                        }
                    }
                }
            })
        }
        "drag" => {
            let (x1, y1, x2, y2) = (number()?, number()?, number()?, number()?);
            run(move || {
                let held = HELD.get();
                mouse(MOVED, x1, y1, 0, held);
                mouse(LEFT_DOWN, x1, y1, 1, held);
                for step in 1..=DRAG_STEPS {
                    let along = f64::from(step) / f64::from(DRAG_STEPS);
                    let (x, y) = (x1 + (x2 - x1) * along, y1 + (y2 - y1) * along);
                    mouse(LEFT_DRAGGED, x, y, 1, held);
                }
                mouse(LEFT_UP, x2, y2, 1, held);
            })
        }
        "hold" => {
            let held = if rest == "none" {
                0
            } else {
                events::flags(&rest)?
            };
            run(move || HELD.set(held))
        }
        "scroll" => {
            let (x, y, dx, dy) = (number()?, number()?, number()?, number()?);
            run(move || events::scroll(x, y, dx as i32, dy as i32))
        }
        "key" | "cmd-key" | "shift-key" | "keys" => {
            let flags = match verb {
                "keys" => events::flags(words.next()?)?,
                "cmd-key" => 1 << 20,
                "shift-key" => 1 << 17,
                _ => 0,
            };
            let code = words.next()?.parse::<u16>().ok()?;
            let characters = match (words.next(), named_characters(code)) {
                (Some(given), _) => given,
                (None, Some(named)) => named,
                (None, None) => "",
            }
            .to_owned();
            run(move || events::key(code, flags, &characters, &characters))
        }
        "type" => run(move || events::type_text(&rest)),
        "drop" => {
            let (path, at) = rest.split_once(char::is_whitespace)?;
            let mut at = at.split_whitespace().map(str::parse::<f32>);
            let (x, y) = (at.next()?.ok()?, at.next()?.ok()?);
            let path = PathBuf::from(path);
            run(move || crate::view::drop_files(vec![path], glam::Vec2::new(x, y)))
        }
        "paste-image" => Step::PasteImage(PathBuf::from(rest)),
        "choose" => run(move || choose(&rest)),
        "menu-dump" => run(move || write(&rest, &menu::dump())),
        "menu-choose" => run(move || menu::choose(&rest)),
        "state" => run(move || write(&rest, &state())),
        "shot" => run(move || shot(&rest)),
        "quit" => Step::Quit,
        _ => return None,
    };
    Some(step)
}

/// Runs the script in `SPECULAR_SHELL_SCRIPT`, if there is one.
pub(crate) fn run_from_env(_window: AnyWindowHandle, cx: &mut App) {
    let Ok(script) = std::env::var("SPECULAR_SHELL_SCRIPT") else {
        return;
    };
    cx.spawn(async move |cx| {
        for text in script
            .split(';')
            .map(str::trim)
            .filter(|step| !step.is_empty())
        {
            match parse(text) {
                Some(Step::Wait(time)) => cx.background_executor().timer(time).await,
                Some(Step::Run(run)) => run(),
                Some(Step::PasteImage(path)) => cx.update(|cx| {
                    shell::with_view(cx, move |view, _, cx| {
                        if !view.paste_image_file(&path, cx) {
                            tracing::warn!(?path, "scripted input: not an image a thread takes");
                        }
                    });
                }),
                Some(Step::Quit) => cx.update(shell::begin_exit),
                None => tracing::warn!(step = text, "scripted input: not a step"),
            }
        }
    })
    .detach();
}
