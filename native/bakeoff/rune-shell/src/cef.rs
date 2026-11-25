//! Real pages: `specular-cef`'s page source under Rune (step S5,
//! `--features cef`, `--source cef`).
//!
//! CEF's message loop is turned by `specular-cef`'s own timer on the main
//! run loop, which Rune's event loop (winit's `NSApplication`) runs: the
//! timer fires between Rune's handlers, never inside one. A frame CEF
//! delivers during that turn waits in the source's queue, and nothing in
//! Rune polls an idle window, so a run-loop observer takes the queue just
//! before the loop sleeps and wakes Rune through the `Waker` from
//! `AppSpec::setup`. The next poll hands the frames to the compositor.
//!
//! A headless run has no run loop going. With `--pump timer` (the default)
//! each poll spins the main run loop briefly, which fires the same timer
//! and the same observer; with `--pump caller` the shell calls
//! `PageSource::pump` each poll, as the real shell's headless run does.

use std::cell::RefCell;
use std::ffi::c_void;
use std::time::{Duration, Instant};

use objc2_core_foundation::{
    CFRunLoop, CFRunLoopActivity, CFRunLoopObserver, kCFRunLoopCommonModes, kCFRunLoopDefaultMode,
};
use rune::Waker;
use specular_cef::{CefConfig, CefPageSource, Pump};
use specular_core::PageEvent;

use crate::pages::{Inbox, Pages, PumpKind, SharedSource};
use crate::shell::Shell;

/// How long a headless run waits for every page to load and paint.
const SETTLE: Duration = Duration::from_secs(10);
/// How long a headless poll lets the main run loop turn.
const SPIN: f64 = 0.004;

/// CEF subprocess entry, first thing in `main`: in a bundle whose helpers
/// are copies of this binary, Chromium's children start here.
pub fn run_subprocess_if_needed() -> Option<i32> {
    specular_cef::run_subprocess_if_needed()
}

/// Makes the process's main bundle on this thread, before anything else
/// can. Rune starts its GPU on a thread (`rune-gpu-prewarm`) as the app
/// starts, and Metal's device list asks for `[NSBundle mainBundle]`; made
/// for the first time there while this thread loads the CEF framework, it
/// crashed in `CFRelease` (2 runs in about 40). Call first in `main`.
pub fn make_main_bundle() {
    // SAFETY: `+[NSBundle mainBundle]` takes no arguments and returns an
    // autoreleased object, which is not kept.
    let _: *mut objc2::runtime::AnyObject =
        unsafe { objc2::msg_send![objc2::class!(NSBundle), mainBundle] };
}

/// Starts CEF and returns its pages. Rune has made its `NSApplication` by
/// the time `App::init` runs, as CEF wants.
pub fn start(pump: PumpKind, headless: bool) -> Result<Pages, String> {
    let config = CefConfig {
        remote_debugging_port: None,
        cache_path: None,
        shared_texture: true,
        pump: match pump {
            PumpKind::Timer => Pump::RunLoopTimer,
            PumpKind::Caller => Pump::Caller,
        },
    };
    let source = CefPageSource::new(config).map_err(|error| format!("starting CEF: {error}"))?;
    let mut pages = Pages::new(Box::new(source), pump);
    pages.spin = headless && pump == PumpKind::Timer;
    Ok(pages)
}

/// What the observer reaches: the source, where it puts what it took, and
/// whom it wakes.
struct Watch {
    source: SharedSource,
    inbox: Inbox,
    waker: Waker,
    wakes: u64,
}

thread_local! {
    static WATCH: RefCell<Option<Watch>> = const { RefCell::new(None) };
}

/// Wakes Rune whenever a page has reported something, by taking the
/// source's queue each time the main run loop is about to sleep.
pub fn watch_paints(pages: &Pages, waker: Waker) {
    let (source, inbox) = pages.shared();
    let first = WATCH.with_borrow_mut(|watch| {
        let first = watch.is_none();
        *watch = Some(Watch {
            source,
            inbox,
            waker,
            wakes: 0,
        });
        first
    });
    if !first {
        return;
    }
    // SAFETY: the callout matches `CFRunLoopObserverCallBack` and uses no
    // context, so a null context pointer is valid.
    let observer = unsafe {
        CFRunLoopObserver::new(
            None,
            CFRunLoopActivity::BeforeWaiting.0,
            true,
            // After winit's own observers, which decide the loop's wait.
            i64::MAX as _,
            Some(observe),
            std::ptr::null_mut(),
        )
    };
    let (Some(observer), Some(run_loop)) = (observer, CFRunLoop::main()) else {
        eprintln!("rune-shell: no run-loop observer; pages paint only on input");
        return;
    };
    // SAFETY: reading an immutable CoreFoundation constant.
    let modes = unsafe { kCFRunLoopCommonModes };
    run_loop.add_observer(Some(&observer), modes);
    // The run loop keeps its own reference; one observer a process.
    std::mem::forget(observer);
}

unsafe extern "C-unwind" fn observe(
    _observer: *mut CFRunLoopObserver,
    _activity: CFRunLoopActivity,
    _info: *mut c_void,
) {
    let _ = WATCH.try_with(|watch| {
        let Ok(mut watch) = watch.try_borrow_mut() else {
            return;
        };
        let Some(watch) = watch.as_mut() else {
            return;
        };
        // Inside a handler that holds the source (a nested run-loop turn),
        // the next poll takes the queue itself.
        let Ok(mut source) = watch.source.try_borrow_mut() else {
            return;
        };
        let Ok(mut inbox) = watch.inbox.try_borrow_mut() else {
            return;
        };
        let before = inbox.len();
        source.drain_events(&mut inbox);
        if inbox.len() > before {
            watch.wakes += 1;
            (watch.waker)();
        }
    });
}

/// How many times the observer woke Rune.
pub fn wakes() -> u64 {
    WATCH.with_borrow(|watch| watch.as_ref().map_or(0, |watch| watch.wakes))
}

/// A headless run's stand-in for a window's run loop: lets timers and
/// observers on the main run loop fire for a moment.
pub fn spin() {
    // SAFETY: reading an immutable CoreFoundation constant.
    let mode = unsafe { kCFRunLoopDefaultMode };
    CFRunLoop::run_in_mode(mode, SPIN, false);
}

/// A headless run: turns the pages until each has painted a frame, so the
/// first snap shows them, as `specular --snapshot --source cef` waits.
pub fn settle(shell: &mut Shell) {
    let started = Instant::now();
    let mut painted = std::collections::HashSet::new();
    while started.elapsed() < SETTLE && painted.len() < shell.hosts.len() {
        if shell.pages.spin {
            spin();
        } else {
            std::thread::sleep(Duration::from_millis(2));
        }
        shell.turn(0.0);
        for event in &shell.page_events {
            if let PageEvent::Frame(frame) = event {
                painted.insert(frame.page);
            }
        }
    }
    eprintln!(
        "rune-shell: {} of {} pages painted in {} ms",
        painted.len(),
        shell.hosts.len(),
        started.elapsed().as_millis()
    );
}
