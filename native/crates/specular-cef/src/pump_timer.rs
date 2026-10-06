//! Drives CEF's external message pump from the main run loop (macOS).
//!
//! `CefDoMessageLoopWork` spins a nested run-loop turn. Called from inside a
//! winit handler, that turn delivers winit's queued events re-entrantly and
//! winit panics ("tried to handle event while another event is currently
//! being handled"). A run-loop timer fires between winit's handlers, where
//! the nested turn is safe.
//!
//! `cef::shutdown` spins the run loop too, and winit's run-loop observers
//! panic once its event loop has returned, so the timer also stops CEF, on
//! request, while the loop still runs.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use objc2_core_foundation::{
    CFAbsoluteTimeGetCurrent, CFRetained, CFRunLoop, CFRunLoopTimer, kCFRunLoopCommonModes,
};

use crate::error::CefError;

/// Half a 120 Hz frame, so a paint never waits a whole frame for the pump.
const INTERVAL_SECONDS: f64 = 1.0 / 240.0;

/// Set while the callback runs: the nested run-loop turn must not pump again.
static PUMPING: AtomicBool = AtomicBool::new(false);

const RUNNING: u8 = 0;
const STOP_REQUESTED: u8 = 1;
const STOPPED: u8 = 2;
static STATE: AtomicU8 = AtomicU8::new(RUNNING);

/// A repeating main-run-loop timer calling `CefDoMessageLoopWork`. Stops on
/// drop; drop it before calling `cef::shutdown` directly.
#[derive(Debug)]
pub(crate) struct PumpTimer(CFRetained<CFRunLoopTimer>);

impl PumpTimer {
    /// Starts pumping. Call on the main thread, after `cef::initialize`.
    pub(crate) fn start() -> Result<Self, CefError> {
        let run_loop = CFRunLoop::main().ok_or(CefError::PumpTimer)?;
        // SAFETY: the callout matches `CFRunLoopTimerCallBack` and uses no
        // context, so a null context pointer is valid.
        let timer = unsafe {
            CFRunLoopTimer::new(
                None,
                CFAbsoluteTimeGetCurrent() + INTERVAL_SECONDS,
                INTERVAL_SECONDS,
                0,
                0,
                Some(pump),
                std::ptr::null_mut(),
            )
        }
        .ok_or(CefError::PumpTimer)?;
        // SAFETY: reading an immutable CoreFoundation constant.
        let common_modes = unsafe { kCFRunLoopCommonModes };
        run_loop.add_timer(Some(&timer), common_modes);
        STATE.store(RUNNING, Ordering::Release);
        Ok(Self(timer))
    }

    /// Asks the next timer fire to stop CEF instead of pumping it. Returns
    /// whether this call made the request (`false` when already asked).
    pub(crate) fn request_stop() -> bool {
        STATE
            .compare_exchange(RUNNING, STOP_REQUESTED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Whether a requested stop has finished: CEF is shut down.
    pub(crate) fn stopped() -> bool {
        STATE.load(Ordering::Acquire) == STOPPED
    }
}

impl Drop for PumpTimer {
    fn drop(&mut self) {
        self.0.invalidate();
    }
}

unsafe extern "C-unwind" fn pump(_timer: *mut CFRunLoopTimer, _info: *mut c_void) {
    if PUMPING.swap(true, Ordering::AcqRel) {
        return;
    }
    match STATE.load(Ordering::Acquire) {
        RUNNING => cef::do_message_loop_work(),
        STOP_REQUESTED => {
            crate::source::stop_cef();
            STATE.store(STOPPED, Ordering::Release);
        }
        _ => {}
    }
    PUMPING.store(false, Ordering::Release);
}
