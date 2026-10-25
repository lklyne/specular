//! What a script asks of the window itself: its size, full screen, and the
//! text input client an input method talks to.

use std::fmt::Write as _;

use objc2::runtime::Bool;
use objc2::{class, msg_send};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::window_facts;
use crate::native::Id;

/// `NSWindowStyleMaskFullScreen`.
const FULL_SCREEN: usize = 1 << 14;
/// `NSNotFound`, the location of a range that names no text.
const NOT_FOUND: usize = isize::MAX as usize;

/// An `NSRange`.
#[repr(C)]
#[derive(Clone, Copy)]
struct Range {
    location: usize,
    length: usize,
}

// SAFETY: the layout is `NSRange`'s, and the encoding is the one GPUI's
// view declares for it, which the debug checks of a message send compare.
unsafe impl objc2::Encode for Range {
    const ENCODING: objc2::Encoding =
        objc2::Encoding::Struct("NSRange", &[usize::ENCODING, usize::ENCODING]);
}

fn window() -> Option<Id> {
    let (number, _) = window_facts()?;
    // SAFETY: the shared application exists while GPUI runs.
    let window: Id = unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        msg_send![app, windowWithWindowNumber: number]
    };
    (!window.is_null()).then_some(window)
}

/// The view the keys go to, which is who an input method talks to.
fn text_client() -> Option<Id> {
    let window = window()?;
    // SAFETY: a live window's first responder, asked whether it is a text
    // input client before it is used as one.
    unsafe {
        let responder: Id = msg_send![window, firstResponder];
        if responder.is_null() {
            return None;
        }
        let is_client: Bool =
            msg_send![responder, respondsToSelector: objc2::sel!(insertText:replacementRange:)];
        is_client.as_bool().then_some(responder)
    }
}

/// Resizes the window so its content is `width` by `height` points, with
/// its top-left corner where it was, as a resize from the bottom-right does.
pub(super) fn resize(width: f64, height: f64) {
    let Some(window) = window() else {
        return;
    };
    // SAFETY: documented calls on a live window, on the main thread.
    unsafe {
        let old: NSRect = msg_send![window, frame];
        let content = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
        let mut frame: NSRect = msg_send![window, frameRectForContentRect: content];
        frame.origin.x = old.origin.x;
        frame.origin.y = old.origin.y + old.size.height - frame.size.height;
        let _: () = msg_send![window, setFrame: frame, display: true, animate: false];
    }
}

/// Moves the window so its frame's bottom-left corner is at `(x, y)` in
/// screen points, which is how it gets to another display.
pub(super) fn move_to(x: f64, y: f64) {
    let Some(window) = window() else {
        return;
    };
    // SAFETY: a documented call on a live window, on the main thread.
    unsafe {
        let _: () = msg_send![window, setFrameOrigin: NSPoint::new(x, y)];
    }
}

/// Each display as a line of a state dump: its frame in screen points and
/// its scale.
fn screens(out: &mut String) {
    // SAFETY: reading the screens `AppKit` lists, on the main thread.
    unsafe {
        let screens: Id = msg_send![class!(NSScreen), screens];
        let count: usize = msg_send![screens, count];
        for index in 0..count {
            let screen: Id = msg_send![screens, objectAtIndex: index];
            let frame: NSRect = msg_send![screen, frame];
            let scale: f64 = msg_send![screen, backingScaleFactor];
            let _ = writeln!(
                out,
                "screen {:.0} {:.0} {:.0} {:.0} scale {scale}",
                frame.origin.x, frame.origin.y, frame.size.width, frame.size.height
            );
        }
    }
}

/// Enters full screen, or leaves it.
pub(super) fn toggle_full_screen() {
    let Some(window) = window() else {
        return;
    };
    let nil: Id = std::ptr::null_mut();
    // SAFETY: a documented call on a live window, on the main thread.
    unsafe {
        let _: () = msg_send![window, toggleFullScreen: nil];
    }
}

/// The input method shows `text` as its composition, the caret at its end.
pub(super) fn set_marked_text(text: &str) -> Result<(), String> {
    let client = text_client().ok_or("nothing in the window takes text input")?;
    let selected = Range {
        location: text.encode_utf16().count(),
        length: 0,
    };
    let replace = Range {
        location: NOT_FOUND,
        length: 0,
    };
    let text = NSString::from_str(text);
    // SAFETY: the responder implements `NSTextInputClient`, checked above.
    unsafe {
        let _: () = msg_send![client,
            setMarkedText: &*text, selectedRange: selected, replacementRange: replace];
    }
    Ok(())
}

/// The input method commits `text`.
pub(super) fn insert_text(text: &str) -> Result<(), String> {
    let client = text_client().ok_or("nothing in the window takes text input")?;
    let replace = Range {
        location: NOT_FOUND,
        length: 0,
    };
    let text = NSString::from_str(text);
    // SAFETY: as above.
    unsafe {
        let _: () = msg_send![client, insertText: &*text, replacementRange: replace];
    }
    Ok(())
}

/// Where the app says the candidate window goes: the rect it answers an
/// input method with, in points from the content's top-left corner.
fn candidate_rect() -> Option<NSRect> {
    let (client, window) = (text_client()?, window()?);
    let (_, height) = window_facts()?;
    let asked = Range {
        location: 0,
        length: 0,
    };
    // GPUI's view declares the range it would fill in as an object and
    // never writes it, so it is given none.
    let actual: Id = std::ptr::null_mut();
    // SAFETY: the responder implements `NSTextInputClient`. The answer is
    // in screen points.
    unsafe {
        let on_screen: NSRect = msg_send![client,
            firstRectForCharacterRange: asked, actualRange: actual];
        let mut rect: NSRect = msg_send![window, convertRectFromScreen: on_screen];
        rect.origin.y = height - rect.origin.y - rect.size.height;
        Some(rect)
    }
}

/// The window as lines of a state dump: its frame on screen, its content
/// size, whether it is full screen or zoomed, the display's scale, and the
/// rect an input method would be given.
pub(super) fn describe(out: &mut String) {
    let Some(window) = window() else {
        return;
    };
    // SAFETY: reading a live window, on the main thread.
    let (frame, content, mask, zoomed, scale) = unsafe {
        let frame: NSRect = msg_send![window, frame];
        let view: Id = msg_send![window, contentView];
        let content: NSRect = msg_send![view, bounds];
        let mask: usize = msg_send![window, styleMask];
        let zoomed: Bool = msg_send![window, isZoomed];
        let scale: f64 = msg_send![window, backingScaleFactor];
        (frame, content, mask, zoomed.as_bool(), scale)
    };
    let _ = writeln!(
        out,
        "window-frame {:.0} {:.0} {:.0} {:.0}",
        frame.origin.x, frame.origin.y, frame.size.width, frame.size.height
    );
    let _ = writeln!(
        out,
        "window-content {:.0} {:.0}",
        content.size.width, content.size.height
    );
    let _ = writeln!(out, "window-full-screen {}", mask & FULL_SCREEN != 0);
    let _ = writeln!(out, "window-zoomed {zoomed}");
    let _ = writeln!(out, "window-scale {scale}");
    screens(out);
    match candidate_rect() {
        Some(rect) => {
            let _ = writeln!(
                out,
                "ime-rect {:.1} {:.1} {:.1} {:.1}",
                rect.origin.x, rect.origin.y, rect.size.width, rect.size.height
            );
        }
        None => {
            let _ = writeln!(out, "ime-rect none");
        }
    }
}
