//! Makes winit's `NSApplication` satisfy CEF's `CefAppProtocol` (macOS).
//!
//! CEF requires `NSApp` to implement `CefAppProtocol` (`cef_app_mac.h`):
//! Chromium casts `NSApp` to `CrAppControlProtocol` and sends it
//! `isHandlingSendEvent` / `setHandlingSendEvent:` (`ScopedSendingEvent`,
//! `MessagePumpCrApplication`). winit owns `NSApp` here (it must be created
//! before CEF initialises), and its subclass knows neither selector, so the
//! first such send would die with "unrecognized selector". [`install`] adds
//! both methods and the three protocol conformances to `NSApp`'s runtime
//! class instead of replacing winit's subclass.

use std::ffi::CString;
use std::sync::atomic::{AtomicBool, Ordering};

use cef::application_mac::{CefAppProtocol, CrAppControlProtocol, CrAppProtocol};
use objc2::runtime::{AnyClass, AnyObject, AnyProtocol, Bool, Imp, Sel};
use objc2::{Encode, ProtocolType, class, ffi, msg_send, sel};

use crate::error::CefError;

/// The one `NSApp`'s `handlingSendEvent` flag.
static HANDLING_SEND_EVENT: AtomicBool = AtomicBool::new(false);

extern "C-unwind" fn is_handling_send_event(_this: *mut AnyObject, _cmd: Sel) -> Bool {
    Bool::new(HANDLING_SEND_EVENT.load(Ordering::Relaxed))
}

extern "C-unwind" fn set_handling_send_event(_this: *mut AnyObject, _cmd: Sel, handling: Bool) {
    HANDLING_SEND_EVENT.store(handling.as_bool(), Ordering::Relaxed);
}

/// Adds `CefAppProtocol` (and its parents) to `NSApp`'s class. Call on the
/// main thread after winit created `NSApp` and the CEF framework is loaded,
/// before `cef_initialize`.
pub(crate) fn install() -> Result<(), CefError> {
    // SAFETY: `+[NSApplication sharedApplication]` takes no arguments and
    // returns the (possibly subclassed) shared instance; we are on the main
    // thread, as AppKit requires.
    let app: *mut AnyObject = unsafe { msg_send![class!(NSApplication), sharedApplication] };
    if app.is_null() {
        return Err(CefError::AppProtocol("NSApp does not exist"));
    }
    // SAFETY: `app` is a live, non-null Objective-C object.
    let class: *const AnyClass = unsafe { ffi::object_getClass(app) };
    // SAFETY: a live object always has a class.
    let class_ref: &AnyClass = unsafe { &*class };

    let bool_type = Bool::ENCODING.to_string();
    let get_types = method_types(&bool_type, "")?;
    let set_types = method_types("v", &bool_type)?;
    // SAFETY: the transmutes erase the signature to the runtime's untyped
    // `IMP`; `get_types` / `set_types` describe exactly these signatures
    // (`BOOL (id, SEL)` and `void (id, SEL, BOOL)`), so the runtime calls
    // them with matching arguments.
    let get_imp = unsafe {
        std::mem::transmute::<extern "C-unwind" fn(*mut AnyObject, Sel) -> Bool, Imp>(
            is_handling_send_event,
        )
    };
    // SAFETY: as above.
    let set_imp = unsafe {
        std::mem::transmute::<extern "C-unwind" fn(*mut AnyObject, Sel, Bool), Imp>(
            set_handling_send_event,
        )
    };
    add_method(
        class,
        class_ref,
        sel!(isHandlingSendEvent),
        get_imp,
        &get_types,
    )?;
    add_method(
        class,
        class_ref,
        sel!(setHandlingSendEvent:),
        set_imp,
        &set_types,
    )?;

    // Parents first, so `CefAppProtocol` conformance implies theirs.
    for protocol in [
        <dyn CrAppProtocol as ProtocolType>::protocol(),
        <dyn CrAppControlProtocol as ProtocolType>::protocol(),
        <dyn CefAppProtocol as ProtocolType>::protocol(),
    ] {
        let Some(protocol) = protocol else {
            // Only referenced protocols are registered at runtime; Chromium
            // sends the selectors whether or not NSApp formally conforms.
            tracing::debug!("CEF app protocol not registered at runtime; methods added only");
            continue;
        };
        add_protocol(class, class_ref, protocol);
    }
    if let Some(cef_app) = <dyn CefAppProtocol as ProtocolType>::protocol()
        && !class_ref.conforms_to(cef_app)
    {
        return Err(CefError::AppProtocol(
            "NSApp does not conform to CefAppProtocol",
        ));
    }
    // The flag must read back through the real message path, not just the
    // runtime tables, or Chromium's first send would still fail.
    // SAFETY: `app` is live and its class now implements both selectors
    // with these exact signatures (`add_method` checked).
    let app_ref: &AnyObject = unsafe { &*app };
    // SAFETY: as above.
    let handling: Bool = unsafe { msg_send![app_ref, isHandlingSendEvent] };
    if handling.as_bool() {
        return Err(CefError::AppProtocol("isHandlingSendEvent started true"));
    }
    Ok(())
}

/// `<ret>@:<args>`: the Objective-C type string for a method on `id`.
fn method_types(ret: &str, args: &str) -> Result<CString, CefError> {
    CString::new(format!("{ret}@:{args}"))
        .map_err(|_| CefError::AppProtocol("method type encoding contains NUL"))
}

fn add_method(
    class: *const AnyClass,
    class_ref: &AnyClass,
    name: Sel,
    imp: Imp,
    types: &CString,
) -> Result<(), CefError> {
    if class_ref.responds_to(name) {
        return Ok(());
    }
    // SAFETY: `class` is a registered class, `types` is a NUL-terminated
    // encoding matching `imp`'s real signature (see `install`).
    let added = unsafe { ffi::class_addMethod(class.cast_mut(), name, imp, types.as_ptr()) };
    if added.as_bool() && class_ref.responds_to(name) {
        Ok(())
    } else {
        Err(CefError::AppProtocol(
            "could not add a CefAppProtocol method",
        ))
    }
}

fn add_protocol(class: *const AnyClass, class_ref: &AnyClass, protocol: &AnyProtocol) {
    if class_ref.conforms_to(protocol) {
        return;
    }
    // SAFETY: `class` is a registered class and `protocol` a registered
    // protocol; adding conformance to an existing class is allowed at runtime.
    let _ = unsafe { ffi::class_addProtocol(class.cast_mut(), protocol) };
}
