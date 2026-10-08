//! What the window exposes to assistive apps, read in-process through the
//! `NSAccessibility` methods a screen reader calls.

use std::fmt::Write as _;

use objc2::runtime::{Bool, Sel};
use objc2::{class, msg_send, sel};
use objc2_foundation::NSRect;

use super::window_facts;
use crate::native::{Id, string_of};

/// How deep the dump goes. GPUI's tree is shallow, and a cycle must end.
const DEPTH: usize = 12;

fn responds(object: Id, selector: Sel) -> bool {
    // SAFETY: `object` is a live Objective-C object.
    let responds: Bool = unsafe { msg_send![object, respondsToSelector: selector] };
    responds.as_bool()
}

/// A string-valued accessibility attribute, or nothing when the element
/// does not have it.
fn text(element: Id, selector: Sel) -> String {
    if !responds(element, selector) {
        return String::new();
    }
    // SAFETY: the selector is one of the `NSAccessibility` getters that
    // return an `NSString` or nil, and the element answers it.
    let value: Id = unsafe { msg_send![element, performSelector: selector] };
    // SAFETY: a live object, asked its class before it is read as a string.
    let is_string: Bool = unsafe {
        if value.is_null() {
            return String::new();
        }
        msg_send![value, isKindOfClass: class!(NSString)]
    };
    if is_string.as_bool() {
        string_of(value)
    } else {
        String::new()
    }
}

fn line(element: Id, depth: usize, out: &mut String) {
    // SAFETY: `element` is a live object from an accessibility array, and
    // `className` returns an `NSString`.
    let class = string_of(unsafe { msg_send![element, className] });
    let is_element = responds(element, sel!(isAccessibilityElement)) && {
        // SAFETY: the element answers this selector.
        let is: Bool = unsafe { msg_send![element, isAccessibilityElement] };
        is.as_bool()
    };
    let frame = if responds(element, sel!(accessibilityFrame)) {
        // SAFETY: the element answers this selector with an `NSRect`.
        let frame: NSRect = unsafe { msg_send![element, accessibilityFrame] };
        format!(
            " frame {:.0} {:.0} {:.0} {:.0}",
            frame.origin.x, frame.origin.y, frame.size.width, frame.size.height
        )
    } else {
        String::new()
    };
    let _ = writeln!(
        out,
        "{}{class} role={:?} label={:?} title={:?} element={is_element}{frame}",
        "  ".repeat(depth),
        text(element, sel!(accessibilityRole)),
        text(element, sel!(accessibilityLabel)),
        text(element, sel!(accessibilityTitle)),
    );
    if depth >= DEPTH || !responds(element, sel!(accessibilityChildren)) {
        return;
    }
    // SAFETY: the element answers this selector with an `NSArray` or nil,
    // whose members live as long as it does.
    unsafe {
        let children: Id = msg_send![element, accessibilityChildren];
        if children.is_null() {
            return;
        }
        let count: usize = msg_send![children, count];
        for index in 0..count {
            let child: Id = msg_send![children, objectAtIndex: index];
            line(child, depth + 1, out);
        }
    }
}

/// The window's accessibility tree, one element a line.
pub(super) fn dump() -> String {
    let mut out = String::new();
    let Some((number, _)) = window_facts() else {
        return out;
    };
    // SAFETY: the shared application exists while GPUI runs.
    let window: Id = unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        msg_send![app, windowWithWindowNumber: number]
    };
    if !window.is_null() {
        line(window, 0, &mut out);
    }
    out
}
