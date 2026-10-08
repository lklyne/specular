//! The refresh rate of the display a window is on, which a benchmark steps
//! its gestures at.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "AppKit calls come in runs on the same live objects; one SAFETY note covers a run"
)]

use std::time::Duration;

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

/// The shortest refresh interval of the display showing the window numbered
/// `window_number`, or `None` when the window or its display cannot be
/// found.
pub(crate) fn interval(window_number: isize) -> Option<Duration> {
    // SAFETY: main thread. `windowWithWindowNumber:` and `screen` return
    // nil or a live object, both are checked before use, and
    // `maximumFramesPerSecond` (macOS 12) returns an `NSInteger`.
    let hz: isize = unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let window: *mut AnyObject = msg_send![app, windowWithWindowNumber: window_number];
        if window.is_null() {
            return None;
        }
        let screen: *mut AnyObject = msg_send![window, screen];
        if screen.is_null() {
            return None;
        }
        msg_send![screen, maximumFramesPerSecond]
    };
    (hz > 0).then(|| Duration::from_secs_f64(1.0 / hz as f64))
}
