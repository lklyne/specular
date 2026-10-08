//! Scripted input for a run nobody is sitting at: an agent checking the
//! window, or a capture for an ADR.
//!
//! `SPECULAR_SHELL_SCRIPT` holds steps separated by `;`. Each is posted to
//! the application's own event queue, so it takes the path a hardware
//! event takes from `sendEvent:` on. Positions are in points from the
//! content's top-left corner.
//!
//! ```text
//! wait 1500; click 800 22; wait 400; shot /tmp/toolbar.png; quit
//! ```
//!
//! Steps: `wait MS`, `move X Y`, `press X Y`, `release X Y`, `click X Y`,
//! `double-click X Y`, `key CODE [CHARS]` (a `kVK_*` code), `cmd-key CODE
//! [CHARS]`, `shot PATH` (the window as the window server composites it)
//! and `quit`.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "each block builds one NSEvent and posts it"
)]

use std::process::Command;
use std::time::Duration;

use gpui_kit::{AnyWindowHandle, App};
use objc2::{class, msg_send};
use objc2_foundation::{NSPoint, NSString};

use crate::canvas;
use crate::native::Id;
use crate::shell;

/// `NSEventType` values.
const LEFT_DOWN: usize = 1;
const LEFT_UP: usize = 2;
const MOVED: usize = 5;
const KEY_DOWN: usize = 10;
const KEY_UP: usize = 11;
/// `NSEventModifierFlagCommand`.
const COMMAND: usize = 1 << 20;

/// The window's number and its content height in points.
fn window_facts() -> Option<(isize, f64)> {
    canvas::with(|canvas| {
        let native = canvas.runtime.window()?.native();
        Some((native.window_number(), native.content_size().1))
    })
    .flatten()
}

fn uptime() -> f64 {
    // SAFETY: `NSProcessInfo` is always there.
    unsafe {
        let info: Id = msg_send![class!(NSProcessInfo), processInfo];
        msg_send![info, systemUptime]
    }
}

fn post(event: Id) {
    if event.is_null() {
        tracing::warn!("scripted input: AppKit made no event");
        return;
    }
    // SAFETY: posting a valid `NSEvent` to our own application's queue.
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, postEvent: event, atStart: false];
    }
}

fn mouse(kind: usize, x: f64, y: f64, clicks: isize) {
    let Some((number, height)) = window_facts() else {
        return;
    };
    let nil: Id = std::ptr::null_mut();
    // SAFETY: a documented `NSEvent` constructor with valid arguments.
    let event: Id = unsafe {
        msg_send![class!(NSEvent),
            mouseEventWithType: kind,
            location: NSPoint::new(x, height - y),
            modifierFlags: 0usize,
            timestamp: uptime(),
            windowNumber: number,
            context: nil,
            eventNumber: 0isize,
            clickCount: clicks,
            pressure: 1.0f32]
    };
    post(event);
}

fn key(code: u16, flags: usize, characters: &str) {
    let Some((number, _)) = window_facts() else {
        return;
    };
    let characters = NSString::from_str(characters);
    let nil: Id = std::ptr::null_mut();
    for kind in [KEY_DOWN, KEY_UP] {
        // SAFETY: a documented `NSEvent` constructor with valid arguments.
        let event: Id = unsafe {
            msg_send![class!(NSEvent),
                keyEventWithType: kind,
                location: NSPoint::new(0.0, 0.0),
                modifierFlags: flags,
                timestamp: uptime(),
                windowNumber: number,
                context: nil,
                characters: &*characters,
                charactersIgnoringModifiers: &*characters,
                isARepeat: false,
                keyCode: code]
        };
        post(event);
    }
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

/// What one step does, or how long it waits.
enum Step {
    Wait(Duration),
    Run(Box<dyn FnOnce()>),
    Quit,
}

fn parse(step: &str) -> Option<Step> {
    let mut words = step.split_whitespace();
    let verb = words.next()?;
    let mut number = || words.next()?.parse::<f64>().ok();
    let step = match verb {
        "wait" => Step::Wait(Duration::from_millis(number()? as u64)),
        "move" | "press" | "release" | "click" | "double-click" => {
            let (x, y) = (number()?, number()?);
            let verb = verb.to_owned();
            Step::Run(Box::new(move || match verb.as_str() {
                "move" => mouse(MOVED, x, y, 0),
                "press" => mouse(LEFT_DOWN, x, y, 1),
                "release" => mouse(LEFT_UP, x, y, 1),
                "click" => {
                    mouse(MOVED, x, y, 0);
                    mouse(LEFT_DOWN, x, y, 1);
                    mouse(LEFT_UP, x, y, 1);
                }
                _ => {
                    mouse(MOVED, x, y, 0);
                    for clicks in [1, 2] {
                        mouse(LEFT_DOWN, x, y, clicks);
                        mouse(LEFT_UP, x, y, clicks);
                    }
                }
            }))
        }
        "key" | "cmd-key" => {
            let code = number()? as u16;
            let characters = words.next().unwrap_or_default().to_owned();
            let flags = if verb == "cmd-key" { COMMAND } else { 0 };
            Step::Run(Box::new(move || key(code, flags, &characters)))
        }
        "shot" => {
            let path = words.next()?.to_owned();
            Step::Run(Box::new(move || shot(&path)))
        }
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
                Some(Step::Quit) => cx.update(shell::begin_exit),
                None => tracing::warn!(step = text, "scripted input: not a step"),
            }
        }
    })
    .detach();
}
