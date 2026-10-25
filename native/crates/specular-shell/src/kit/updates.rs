//! Updates, through Sparkle.
//!
//! The app bundle carries `Sparkle.framework` and names its feed and its
//! public key in `Info.plist` (`SUFeedURL`, `SUPublicEDKey`). Sparkle checks
//! the feed on its own schedule, downloads the new app, verifies it against
//! the key and swaps it in on the next launch.
//!
//! The framework is loaded at run time and never linked, so a build that is
//! not the bundle (a `cargo run`, the measurement bundle) has no updater
//! and says so only when asked to check.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "AppKit calls come in runs on the same live objects; one SAFETY note covers a run"
)]

use std::cell::OnceCell;
use std::path::PathBuf;

use objc2::runtime::{AnyClass, Bool};
use objc2::{class, msg_send};
use objc2_foundation::NSString;

use crate::native::Id;

/// Where the framework is, from the executable in `Contents/MacOS`.
const FRAMEWORK: &str = "../Frameworks/Sparkle.framework";

thread_local! {
    /// Sparkle's `SPUStandardUpdaterController`, kept for the process's
    /// life, or nothing in a build with no updater.
    static CONTROLLER: OnceCell<Option<Id>> = const { OnceCell::new() };
}

fn framework() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let path = exe.parent()?.join(FRAMEWORK);
    path.is_dir().then_some(path)
}

fn controller() -> Option<Id> {
    let path = framework()?;
    let path = NSString::from_str(&path.to_string_lossy());
    let nil: Id = std::ptr::null_mut();
    // SAFETY: `path` is a live `NSString`, and `bundleWithPath:` and `load`
    // take and return what `NSBundle` declares. The controller class is
    // looked up only after its framework loaded, and its initializer takes
    // a `BOOL` and two delegates, both nil here. The controller is never
    // released, as it runs the updater for as long as the app does.
    unsafe {
        let bundle: Id = msg_send![class!(NSBundle), bundleWithPath: &*path];
        if bundle.is_null() {
            return None;
        }
        let loaded: Bool = msg_send![bundle, load];
        if !loaded.as_bool() {
            return None;
        }
        let class = AnyClass::get(c"SPUStandardUpdaterController")?;
        let controller: Id = msg_send![class, alloc];
        let controller: Id = msg_send![
            controller,
            initWithStartingUpdater: Bool::YES,
            updaterDelegate: nil,
            userDriverDelegate: nil
        ];
        (!controller.is_null()).then_some(controller)
    }
}

fn started() -> Option<Id> {
    CONTROLLER.with(|cell| *cell.get_or_init(controller))
}

/// Starts the updater, which then checks the feed on its own schedule. Does
/// nothing in a build with no updater.
pub(crate) fn start() {
    if started().is_none() {
        tracing::info!("no updater in this build");
    }
}

/// Specular > Check for updates…: checks now and shows what was found.
pub(crate) fn check() {
    let Some(controller) = started() else {
        tracing::warn!("this build has no updater to check with");
        return;
    };
    let nil: Id = std::ptr::null_mut();
    // SAFETY: `controller` is the live controller made above, and
    // `checkForUpdates:` takes a sender, which may be nil.
    let _: () = unsafe { msg_send![controller, checkForUpdates: nil] };
}
