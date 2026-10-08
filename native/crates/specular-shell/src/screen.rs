//! Whether anything this process draws can reach the screen. While the
//! screen is locked or the display sleeps the window server gives the
//! window no frames: the Kit draws its first and the canvas none, so a
//! measurement taken then is of nothing.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "one read of the session's dictionary; one SAFETY note covers it"
)]

use std::ffi::c_void;

use objc2::msg_send;
use objc2_foundation::NSString;

use crate::native::Id;

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    /// A `CFDictionaryRef` the caller owns, or null with no session.
    fn CGSessionCopyCurrentDictionary() -> *const c_void;
    fn CGMainDisplayID() -> u32;
    /// A `boolean_t`.
    fn CGDisplayIsAsleep(display: u32) -> u32;
}

/// Why nothing drawn now would be shown, or `None` when the screen is on
/// and unlocked.
pub(crate) fn why_nothing_shows() -> Option<&'static str> {
    // SAFETY: documented CoreGraphics calls. A `CFDictionary` is an
    // `NSDictionary`, read with `objectForKey:` and released once, as the
    // copy this owns; the value is an `NSNumber` or nil.
    unsafe {
        if CGDisplayIsAsleep(CGMainDisplayID()) != 0 {
            return Some("the display is asleep");
        }
        let session: Id = CGSessionCopyCurrentDictionary().cast_mut().cast();
        if session.is_null() {
            return Some("there is no window server session");
        }
        let key = NSString::from_str("CGSSessionScreenIsLocked");
        let value: Id = msg_send![session, objectForKey: &*key];
        let locked: bool = !value.is_null() && msg_send![value, boolValue];
        let _: () = msg_send![session, release];
        locked.then_some("the screen is locked")
    }
}
