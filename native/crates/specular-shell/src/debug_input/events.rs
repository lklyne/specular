//! The `NSEvent`s a script posts: pointer, key and scroll.

use std::ffi::c_void;

use objc2::{class, msg_send, sel};
use objc2_foundation::{NSPoint, NSRect, NSString};

use super::window_facts;
use crate::native::Id;

/// `NSEventType` values.
pub(super) const LEFT_DOWN: usize = 1;
pub(super) const LEFT_UP: usize = 2;
pub(super) const MOVED: usize = 5;
pub(super) const LEFT_DRAGGED: usize = 6;
const KEY_DOWN: usize = 10;
const KEY_UP: usize = 11;

/// `NSEventModifierFlag` bits by the name a script gives them.
const MODIFIERS: [(&str, usize); 4] = [
    ("shift", 1 << 17),
    ("ctrl", 1 << 18),
    ("alt", 1 << 19),
    ("cmd", 1 << 20),
];

/// The flags of `shift+cmd`, or `None` for a name that is not a modifier.
pub(super) fn flags(names: &str) -> Option<usize> {
    names.split('+').try_fold(0, |flags, name| {
        let (_, bit) = MODIFIERS.iter().find(|(known, _)| *known == name)?;
        Some(flags | bit)
    })
}

/// The `kVK_*` code and whether shift is held for a character of the US
/// layout, which is all `type` knows.
fn key_of(character: char) -> Option<(u16, bool)> {
    const PLAIN: &str = "asdfhgzxcv\u{0}bqweryt123465=97-80]ou[ip\rlj'k;\\,/nm.\t `";
    const SHIFTED: &str = "ASDFHGZXCV\u{0}BQWERYT!@#$^%+(&_*)}OU{IP\rLJ\"K:|<?NM>\t ~";
    let find = |keys: &str| {
        (keys.chars().position(|key| key == character)).and_then(|at| u16::try_from(at).ok())
    };
    find(PLAIN)
        .map(|code| (code, false))
        .or_else(|| find(SHIFTED).map(|code| (code, true)))
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

/// A pointer event of `kind` at `(x, y)` with `held` modifier flags.
pub(super) fn mouse(kind: usize, x: f64, y: f64, clicks: isize, held: usize) {
    let Some((number, height)) = window_facts() else {
        return;
    };
    let nil: Id = std::ptr::null_mut();
    // SAFETY: a documented `NSEvent` constructor with valid arguments.
    let event: Id = unsafe {
        msg_send![class!(NSEvent),
            mouseEventWithType: kind,
            location: NSPoint::new(x, height - y),
            modifierFlags: held,
            timestamp: uptime(),
            windowNumber: number,
            context: nil,
            eventNumber: 0isize,
            clickCount: clicks,
            pressure: 1.0f32]
    };
    post(event);
}

/// A key going down and coming up.
pub(super) fn key(code: u16, flags: usize, characters: &str, plain: &str) {
    let Some((number, _)) = window_facts() else {
        return;
    };
    let characters = NSString::from_str(characters);
    let plain = NSString::from_str(plain);
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
                charactersIgnoringModifiers: &*plain,
                isARepeat: false,
                keyCode: code]
        };
        post(event);
    }
}

/// Types `text` a key at a time. A character the US layout has no key for
/// is skipped with a warning.
pub(super) fn type_text(text: &str) {
    for character in text.chars() {
        let Some((code, shift)) = key_of(character) else {
            tracing::warn!(%character, "scripted input: no key for this character");
            continue;
        };
        let typed = character.to_string();
        let plain = typed.to_lowercase();
        key(code, if shift { 1 << 17 } else { 0 }, &typed, &plain);
    }
}

/// Hands a `CoreGraphics` event to the view under `(x, y)`. `AppKit` makes
/// no right press with its button set and no scroll at all, and a
/// `CoreGraphics` event reaches the process with no window, so this does
/// the window's routing step itself, as `-[NSWindow sendEvent:]` does for a
/// hardware event.
fn deliver(made: *mut CGEvent, x: f64, y: f64, selector: objc2::runtime::Sel) {
    let Some((number, height)) = window_facts() else {
        return;
    };
    if made.is_null() {
        return;
    }
    // SAFETY: `made` is a live event released here, and the AppKit calls
    // are on the live window. With no window the event's `locationInWindow`
    // is its screen point, so the screen point is set to the window point
    // wanted.
    unsafe {
        let screens: Id = msg_send![class!(NSScreen), screens];
        let first: Id = msg_send![screens, firstObject];
        let screen: NSRect = msg_send![first, frame];
        CGEventSetLocation(made, NSPoint::new(x, screen.size.height - (height - y)));
        let event: Id = msg_send![class!(NSEvent), eventWithCGEvent: made];
        CFRelease(made.cast());
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        let window: Id = msg_send![app, windowWithWindowNumber: number];
        let content: Id = msg_send![window, contentView];
        let target: Id = msg_send![content, hitTest: NSPoint::new(x, height - y)];
        if event.is_null() || target.is_null() {
            tracing::warn!("scripted input: no view under the point");
            return;
        }
        let _: Id = msg_send![target, performSelector: selector, withObject: event];
    }
}

/// A pixel scroll at `(x, y)`, as a trackpad sends.
pub(super) fn scroll(x: f64, y: f64, dx: i32, dy: i32) {
    // SAFETY: a documented CoreGraphics constructor; unit 0 is pixels.
    let made = unsafe { CGEventCreateScrollWheelEvent2(std::ptr::null(), 0, 2, dy, dx, 0) };
    deliver(made, x, y, sel!(scrollWheel:));
}

/// A press and release of the right button at `(x, y)`.
pub(super) fn right_click(x: f64, y: f64) {
    for (kind, selector) in [(3, sel!(rightMouseDown:)), (4, sel!(rightMouseUp:))] {
        // SAFETY: a documented CoreGraphics constructor: the right button
        // is 1, and field 1 is the click count.
        let made = unsafe {
            let made = CGEventCreateMouseEvent(std::ptr::null(), kind, NSPoint::new(0.0, 0.0), 1);
            if !made.is_null() {
                CGEventSetIntegerValueField(made, 1, 1);
            }
            made
        };
        deliver(made, x, y, selector);
    }
}

/// A `CGEvent`, which is only ever held by pointer.
#[repr(C)]
struct CGEvent {
    _opaque: [u8; 0],
}

// SAFETY: the encoding is `CGEventRef`'s, a pointer to an opaque struct.
unsafe impl objc2::RefEncode for CGEvent {
    const ENCODING_REF: objc2::Encoding =
        objc2::Encoding::Pointer(&objc2::Encoding::Struct("__CGEvent", &[]));
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateScrollWheelEvent2(
        source: *const c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> *mut CGEvent;
    fn CGEventCreateMouseEvent(
        source: *const c_void,
        kind: u32,
        location: NSPoint,
        button: u32,
    ) -> *mut CGEvent;
    fn CGEventSetIntegerValueField(event: *mut CGEvent, field: u32, value: i64);
    fn CGEventSetLocation(event: *mut CGEvent, location: NSPoint);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(object: *mut c_void);
}
