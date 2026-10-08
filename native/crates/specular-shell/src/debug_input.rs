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
//! `double-click X Y`, `right-click X Y`, `drag X1 Y1 X2 Y2` (a press, the
//! pointer moved across in steps with the button down, a release),
//! `key CODE [CHARS]` (a `kVK_*` code; Return, Tab, Space, Delete, Escape
//! and the arrows carry their own characters), `cmd-key CODE [CHARS]`,
//! `shift-key CODE [CHARS]`, `type TEXT` (the rest of the step, one key
//! press a character on the US layout), `paste-image PATH` (the composer
//! takes the image file as a paste would hand it over; the system
//! pasteboard is not touched), `choose NAME` (the first-run choice of that
//! control name, run as its button runs it: for a run with the screen
//! locked, when nothing is drawn to click on), `shot PATH` (the window as
//! the window server composites it) and `quit`.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "each block builds one NSEvent and posts it"
)]

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use gpui_kit::{AnyWindowHandle, App};
use objc2::{class, msg_send};
use objc2_foundation::{NSPoint, NSString};
use specular_interact::Event;

use crate::canvas;
use crate::native::Id;
use crate::shell;

/// `NSEventType` values.
const LEFT_DOWN: usize = 1;
const LEFT_UP: usize = 2;
const RIGHT_DOWN: usize = 3;
const RIGHT_UP: usize = 4;
const MOVED: usize = 5;
const LEFT_DRAGGED: usize = 6;
const KEY_DOWN: usize = 10;
const KEY_UP: usize = 11;
/// `NSEventModifierFlagShift` and `NSEventModifierFlagCommand`.
const SHIFT: usize = 1 << 17;
const COMMAND: usize = 1 << 20;
/// How many moves a `drag` makes between its ends.
const DRAG_STEPS: u32 = 8;

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

/// Core Graphics' event, which an `NSEvent` wraps.
#[repr(C)]
struct CgEvent {
    _opaque: [u8; 0],
}

// SAFETY: `CGEventRef` is a pointer to the opaque struct `__CGEvent`.
unsafe impl objc2::RefEncode for CgEvent {
    const ENCODING_REF: objc2::Encoding =
        objc2::Encoding::Pointer(&objc2::Encoding::Struct("__CGEvent", &[]));
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventSetIntegerValueField(event: *mut CgEvent, field: u32, value: i64);
}

/// `kCGMouseEventButtonNumber`, and the right button's number.
const BUTTON_NUMBER_FIELD: u32 = 3;
const RIGHT_BUTTON: i64 = 1;

/// `event` as the right button's. `AppKit` makes every synthesized mouse
/// event with button number 0 whatever its type, and GPUI reads the button
/// from the number, so the number is set on the Core Graphics event under
/// it and the `NSEvent` made again from that.
fn as_right_button(event: Id) -> Id {
    if event.is_null() {
        return event;
    }
    // SAFETY: `CGEvent` returns the event's own Core Graphics event, which
    // lives as long as the `NSEvent`; the field is an integer one; and
    // `eventWithCGEvent:` takes such an event.
    unsafe {
        let underlying: *mut CgEvent = msg_send![event, CGEvent];
        if underlying.is_null() {
            return event;
        }
        CGEventSetIntegerValueField(underlying, BUTTON_NUMBER_FIELD, RIGHT_BUTTON);
        msg_send![class!(NSEvent), eventWithCGEvent: underlying]
    }
}

fn mouse(kind: usize, x: f64, y: f64, clicks: isize) {
    let Some((number, height)) = window_facts() else {
        return;
    };
    let nil: Id = std::ptr::null_mut();
    // SAFETY: a documented `NSEvent` constructor with valid arguments.
    let mut event: Id = unsafe {
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
    if matches!(kind, RIGHT_DOWN | RIGHT_UP) {
        event = as_right_button(event);
    }
    post(event);
}

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

/// The key that types `character` on the US layout, and whether Shift is
/// held for it.
fn us_key(character: char) -> Option<(u16, bool)> {
    const PLAIN: &str = "asdfhgzxcv\u{0}bqweryt123465=97-80]ou[ip\rlj'k;\\,/nm.\t `";
    const SHIFTED: &str = "ASDFHGZXCV\u{0}BQWERYT!@#$^%+(&_*)}OU{IP\rLJ\"K:|<?NM>\t ~";
    let find = |keys: &str| keys.chars().position(|key| key == character && key != '\0');
    let (code, shift) = match (find(PLAIN), find(SHIFTED)) {
        (Some(code), _) => (code, false),
        (None, Some(code)) => (code, true),
        (None, None) => return None,
    };
    Some((u16::try_from(code).ok()?, shift))
}

fn type_text(text: &str) {
    for character in text.chars() {
        if let Some((code, shift)) = us_key(character) {
            key(code, if shift { SHIFT } else { 0 }, &character.to_string());
        } else {
            tracing::warn!(%character, "scripted input: no key types this");
        }
    }
}

fn key(code: u16, flags: usize, characters: &str) {
    let Some((number, _)) = window_facts() else {
        return;
    };
    let characters = match named_characters(code) {
        Some(named) if characters.is_empty() => named,
        _ => characters,
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

/// Runs the first-run view's choice named `name`.
fn choose(name: &str) {
    let action = canvas::models()
        .and_then(|models| models.onboarding)
        .and_then(|view| view.choices.into_iter().find(|choice| choice.name == name))
        .map(|choice| choice.action);
    if let Some(action) = action {
        canvas::dispatch(Event::Action(action));
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
    Run(Box<dyn FnOnce()>),
    /// The composer is handed this image file as a paste.
    PasteImage(PathBuf),
    Quit,
}

fn parse(step: &str) -> Option<Step> {
    if let Some(text) = step.strip_prefix("type ") {
        let text = text.to_owned();
        return Some(Step::Run(Box::new(move || type_text(&text))));
    }
    let mut words = step.split_whitespace();
    let verb = words.next()?;
    let mut number = || words.next()?.parse::<f64>().ok();
    let step = match verb {
        "wait" => Step::Wait(Duration::from_millis(number()? as u64)),
        "drag" => {
            let (x1, y1, x2, y2) = (number()?, number()?, number()?, number()?);
            Step::Run(Box::new(move || {
                mouse(MOVED, x1, y1, 0);
                mouse(LEFT_DOWN, x1, y1, 1);
                for step in 1..=DRAG_STEPS {
                    let along = f64::from(step) / f64::from(DRAG_STEPS);
                    mouse(
                        LEFT_DRAGGED,
                        x1 + (x2 - x1) * along,
                        y1 + (y2 - y1) * along,
                        1,
                    );
                }
                mouse(LEFT_UP, x2, y2, 1);
            }))
        }
        "right-click" => {
            let (x, y) = (number()?, number()?);
            Step::Run(Box::new(move || {
                mouse(MOVED, x, y, 0);
                mouse(RIGHT_DOWN, x, y, 1);
                mouse(RIGHT_UP, x, y, 1);
            }))
        }
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
        "key" | "cmd-key" | "shift-key" => {
            let code = number()? as u16;
            let characters = words.next().unwrap_or_default().to_owned();
            let flags = match verb {
                "cmd-key" => COMMAND,
                "shift-key" => SHIFT,
                _ => 0,
            };
            Step::Run(Box::new(move || key(code, flags, &characters)))
        }
        "choose" => {
            let name = words.next()?.to_owned();
            Step::Run(Box::new(move || choose(&name)))
        }
        "shot" => {
            let path = words.next()?.to_owned();
            Step::Run(Box::new(move || shot(&path)))
        }
        "paste-image" => Step::PasteImage(PathBuf::from(words.next()?)),
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
