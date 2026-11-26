//! Drives CEF's external message pump from the main run loop (macOS).
//!
//! `CefDoMessageLoopWork` spins a nested run-loop turn. Called from inside
//! one of the window's event handlers, that turn delivers the window's
//! queued events re-entrantly, inside the handler still running. A run-loop
//! timer fires between the window's handlers, where the nested turn is safe.
//!
//! The timer fires when CEF asks for work
//! (`OnScheduleMessagePumpWork`, see [`schedule`]), at 30 Hz while a page
//! loads ([`hurry`]), and otherwise once a second, so an app whose pages
//! are still does not wake to find nothing to do.
//!
//! `cef::shutdown` spins the run loop too, which must happen while the
//! window's run loop still runs, so the timer also stops CEF, on request,
//! before the app quits.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, Ordering};

use objc2_core_foundation::{
    CFAbsoluteTimeGetCurrent, CFRetained, CFRunLoop, CFRunLoopTimer, kCFRunLoopCommonModes,
};

use crate::config::{HURRY_SECONDS, PUMP_FALLBACK_SECONDS, pump_delay_seconds, pump_moves};
use crate::error::CefError;

/// The running timer, for [`schedule`], which CEF calls from any thread.
/// Null until the timer starts. The timer it points to is never released
/// (see [`PumpTimer::start`]), so a pointer read here is always live.
static TIMER: AtomicPtr<CFRunLoopTimer> = AtomicPtr::new(std::ptr::null_mut());

/// Set while the callback runs: the nested run-loop turn must not pump again.
static PUMPING: AtomicBool = AtomicBool::new(false);

/// Set while a page loads: CEF does not ask for all the work of a load,
/// so the pump keeps a quick pace of its own until loads are done.
static HURRY: AtomicBool = AtomicBool::new(false);

/// Set when the timer fired inside a pump and was skipped: work CEF asked
/// for during the pump, which the pump that is running may not have done.
static MISSED: AtomicBool = AtomicBool::new(false);

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
                CFAbsoluteTimeGetCurrent(),
                PUMP_FALLBACK_SECONDS,
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
        // One reference is leaked on purpose: `schedule` runs on CEF's
        // threads with no lock, and must never find the timer freed. There
        // is one timer a process.
        let kept = CFRetained::into_raw(timer.clone());
        TIMER.store(kept.as_ptr(), Ordering::Release);
        Ok(Self(timer))
    }

    /// Asks the next timer fire to stop CEF instead of pumping it. Returns
    /// whether this call made the request (`false` when already asked).
    pub(crate) fn request_stop() -> bool {
        let asked = STATE
            .compare_exchange(RUNNING, STOP_REQUESTED, Ordering::AcqRel, Ordering::Acquire)
            .is_ok();
        if asked {
            schedule(0);
        }
        asked
    }

    /// Whether a requested stop has finished: CEF is shut down.
    pub(crate) fn stopped() -> bool {
        STATE.load(Ordering::Acquire) == STOPPED
    }
}

impl Drop for PumpTimer {
    fn drop(&mut self) {
        // Setting the fire date of an invalid timer does nothing.
        self.0.invalidate();
    }
}

/// Whether the pump should hurry: a page is loading or an agent is driving
/// one. While it should, the pump runs at least every [`HURRY_SECONDS`].
pub(crate) fn hurry(on: bool) {
    if HURRY.swap(on, Ordering::AcqRel) != on && on {
        schedule((HURRY_SECONDS * 1_000.0) as i64);
    }
}

/// CEF has work for its UI thread in `delay_ms` milliseconds, or now when
/// that is zero or less. Moves the timer's next fire up to then, never
/// later than a fire already due ([`pump_moves`]). Callable from any
/// thread, as `OnScheduleMessagePumpWork` is.
pub(crate) fn schedule(delay_ms: i64) {
    let timer = TIMER.load(Ordering::Acquire);
    // SAFETY: the pointer is null or the timer `PumpTimer::start` leaked a
    // reference to, which is therefore never freed. `CFRunLoopTimer` may be
    // used from any thread.
    let Some(timer) = (unsafe { timer.as_ref() }) else {
        return;
    };
    let now = CFAbsoluteTimeGetCurrent();
    let asked = now + pump_delay_seconds(delay_ms);
    if pump_moves(asked, timer.next_fire_date(), now) {
        timer.set_next_fire_date(asked);
    }
}

unsafe extern "C-unwind" fn pump(timer: *mut CFRunLoopTimer, _info: *mut c_void) {
    if PUMPING.swap(true, Ordering::AcqRel) {
        MISSED.store(true, Ordering::Release);
        return;
    }
    match STATE.load(Ordering::Acquire) {
        RUNNING => {
            let started = specular_core::ledger::now_us();
            cef::do_message_loop_work();
            specular_core::ledger::pump(started);
        }
        STOP_REQUESTED => {
            crate::source::stop_cef();
            STATE.store(STOPPED, Ordering::Release);
        }
        _ => {}
    }
    PUMPING.store(false, Ordering::Release);
    // A fire skipped inside the pump has had its next fire put off by the
    // timer's whole interval; the work it was for runs now instead.
    // SAFETY: CoreFoundation passes the timer that is firing, alive for
    // the callout.
    let Some(timer) = (unsafe { timer.as_ref() }) else {
        return;
    };
    let now = CFAbsoluteTimeGetCurrent();
    let next = if MISSED.swap(false, Ordering::AcqRel) {
        now
    } else if HURRY.load(Ordering::Acquire) {
        now + HURRY_SECONDS
    } else {
        return;
    };
    if pump_moves(next, timer.next_fire_date(), now) {
        timer.set_next_fire_date(next);
    }
}
