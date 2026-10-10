//! Starting the resize ledger ([`specular_core::ledger`]) and the one
//! entry only the shell can make: how long each turn of the main run loop
//! was busy.

use std::ffi::c_void;
use std::sync::atomic::{AtomicU64, Ordering};

use objc2_core_foundation::{
    CFRunLoop, CFRunLoopActivity, CFRunLoopObserver, kCFRunLoopCommonModes,
};
use specular_core::ledger::{self, Entry};

/// When the run loop last woke, in ledger time plus one. Zero while it
/// sleeps.
static WOKE_AT: AtomicU64 = AtomicU64::new(0);

unsafe extern "C-unwind" fn observe(
    _observer: *mut CFRunLoopObserver,
    activity: CFRunLoopActivity,
    _info: *mut c_void,
) {
    if activity == CFRunLoopActivity::AfterWaiting {
        WOKE_AT.store(ledger::now_us() + 1, Ordering::Relaxed);
        return;
    }
    let woke = WOKE_AT.swap(0, Ordering::Relaxed);
    if woke != 0 {
        ledger::record(Entry::Loop((ledger::now_us() + 1).saturating_sub(woke)));
    }
}

/// Turns the ledger on if the environment asks for it, and then watches
/// the main run loop in the common modes, which the live-resize loop's
/// tracking mode is one of. Call on the main thread.
pub(crate) fn start() {
    ledger::init_from_env();
    if !ledger::enabled() {
        return;
    }
    let activities = CFRunLoopActivity::AfterWaiting | CFRunLoopActivity::BeforeWaiting;
    // SAFETY: the callout matches `CFRunLoopObserverCallBack` and uses no
    // context, so a null context pointer is valid.
    let observer = unsafe {
        CFRunLoopObserver::new(
            None,
            activities.0,
            true,
            0,
            Some(observe),
            std::ptr::null_mut(),
        )
    };
    let (Some(observer), Some(run_loop)) = (observer, CFRunLoop::main()) else {
        return;
    };
    // SAFETY: reading an immutable CoreFoundation constant.
    let common_modes = unsafe { kCFRunLoopCommonModes };
    // The run loop keeps the observer for the rest of the run.
    run_loop.add_observer(Some(&observer), common_modes);
}
