//! macOS's Reduce motion, read live and handed to the app.
//!
//! `AppKit` posts no notification this shell already listens for when the
//! setting changes, so it is read again every [`POLL`]: one property read
//! on the shared workspace, cheap enough to do that often. The app holds
//! the value ([`specular_interact::App::reduce_motion`]) and the canvas's
//! transitions, the agent cursors and the Kit's own animations read it
//! there.

use std::time::Duration;

use gpui_kit::App;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use specular_interact::Event;

use crate::canvas;

/// How often the setting is read again.
const POLL: Duration = Duration::from_secs(1);

/// Whether the system asks for reduced motion now.
fn system_reduces_motion() -> bool {
    // SAFETY: `+[NSWorkspace sharedWorkspace]` returns the process's shared
    // workspace, never nil, and `accessibilityDisplayShouldReduceMotion` is a
    // BOOL property of it, available since macOS 10.12 and safe to read from
    // any thread.
    let reduce: Bool = unsafe {
        let workspace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
        msg_send![workspace, accessibilityDisplayShouldReduceMotion]
    };
    reduce.as_bool()
}

/// Tells the app the setting now, and again each time it changes.
pub(crate) fn follow_system(cx: &mut App) {
    let mut told = system_reduces_motion();
    canvas::dispatch(Event::ReduceMotion(told));
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(POLL).await;
            let now = system_reduces_motion();
            if now != told {
                told = now;
                tracing::info!(reduce = now, "Reduce motion changed");
                canvas::dispatch(Event::ReduceMotion(now));
            }
        }
    })
    .detach();
}

/// Whether the app holds its animations still: the system asks for reduced
/// motion. `false` before the canvas exists.
pub(crate) fn reduced() -> bool {
    canvas::with(|canvas| canvas.runtime.app().reduce_motion()).unwrap_or(false)
}
