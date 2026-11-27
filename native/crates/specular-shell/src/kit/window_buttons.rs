//! The window's close, minimize and zoom buttons as a layout asks for them
//! (ADR 0059): where the app puts them, somewhere else, or hidden for a
//! layout whose own title bar draws them and runs `window.close`,
//! `app.minimize` and `window.zoom`.

use std::cell::Cell;

use gpui_kit::{Window, point, px};
use objc2::msg_send;
use objc2::runtime::{AnyObject, Bool};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use specular_interact::TrafficLights;

/// Where the app puts the first button's corner, in points.
pub(crate) const NATIVE: (f32, f32) = (14.0, 13.0);

thread_local! {
    /// What was applied last, so a frame with the same look does nothing.
    static APPLIED: Cell<Option<TrafficLights>> = const { Cell::new(None) };
}

/// The `NSWindow` GPUI's window is.
fn ns_window(window: &Window) -> Option<*mut AnyObject> {
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return None;
    };
    let view: *mut AnyObject = handle.ns_view.as_ptr().cast();
    // SAFETY: the view is GPUI's live content view, read on the main thread.
    let ns_window: *mut AnyObject = unsafe { msg_send![view, window] };
    (!ns_window.is_null()).then_some(ns_window)
}

/// Shows or hides the three buttons.
fn set_hidden(ns_window: *mut AnyObject, hidden: bool) {
    for kind in 0_usize..3 {
        // SAFETY: `standardWindowButton:` with `NSWindowCloseButton`,
        // `NSWindowMiniaturizeButton` and `NSWindowZoomButton` on a live
        // window answers its button or nil, on the main thread.
        let button: *mut AnyObject = unsafe { msg_send![ns_window, standardWindowButton: kind] };
        if !button.is_null() {
            // SAFETY: a documented setter on a live button.
            let _: () = unsafe { msg_send![button, setHidden: Bool::new(hidden)] };
        }
    }
}

/// Puts the buttons where `lights` says, when that changed.
pub(crate) fn apply(lights: TrafficLights, window: &mut Window) {
    if APPLIED.with(Cell::get) == Some(lights) {
        return;
    }
    let Some(ns_window) = ns_window(window) else {
        return;
    };
    APPLIED.with(|applied| applied.set(Some(lights)));
    match lights {
        TrafficLights::Hidden(_) => set_hidden(ns_window, true),
        TrafficLights::Native | TrafficLights::At { .. } => {
            let (x, y) = match lights {
                TrafficLights::At { x, y } => (x, y),
                TrafficLights::Native | TrafficLights::Hidden(_) => NATIVE,
            };
            set_hidden(ns_window, false);
            window.set_traffic_light_position(point(px(x), px(y)));
        }
    }
}

/// Closes the window as its close button does.
pub(crate) fn close(window: &Window) {
    if let Some(ns_window) = ns_window(window) {
        // SAFETY: `performClose:` with a nil sender on a live window, on the
        // main thread.
        let _: () =
            unsafe { msg_send![ns_window, performClose: std::ptr::null_mut::<AnyObject>()] };
    }
}
