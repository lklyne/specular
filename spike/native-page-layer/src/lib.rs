//! Measurement spike: hand IOSurface-backed offscreen page frames straight to
//! Core Animation, zero-copy, bypassing the render-process -> IPC -> canvas
//! renderer copy path.
//!
//! One host `NSView` (layer-backed) is inserted into the Electron window's
//! content view. Each page gets one `CALayer` whose `contents` is set
//! directly to the page's `IOSurfaceRef`. WindowServer composites the
//! IOSurface without any additional copy.
//!
//! All entry points are expected to be called from the process's single
//! main/AppKit thread (Electron main's JS thread). State lives in a
//! `thread_local!` for that reason: nothing here is `Send`, and nothing
//! needs to be.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};

use napi::bindgen_prelude::*;
use napi_derive::napi;
use objc2::runtime::{AnyObject, Bool};
use objc2::{class, msg_send};
use objc2::{Encode, Encoding};

// ---------------------------------------------------------------------------
// CoreGraphics geometry, hand-declared (see spike brief: avoids feature-
// hunting through objc2-quartz-core/objc2-core-foundation for a throwaway).
// CGFloat is `f64` everywhere we run (macOS arm64/x86_64 64-bit only).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct CGPoint {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct CGSize {
    width: f64,
    height: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

// SAFETY: repr(C) layout matches the Objective-C struct definitions exactly
// (two/four `double` fields), and the names match the real CG type names
// used in `@encode`, which is all `Encode` needs to be correct here.
unsafe impl Encode for CGPoint {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl Encode for CGSize {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl Encode for CGRect {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[CGPoint::ENCODING, CGSize::ENCODING]);
}

impl CGRect {
    fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        CGRect {
            origin: CGPoint { x, y },
            size: CGSize { width: w, height: h },
        }
    }
}

// NSWindowOrderingMode
const NS_WINDOW_ABOVE: isize = 1;
const NS_WINDOW_BELOW: isize = -1;

// NSAutoresizingMaskOptions
const NS_VIEW_WIDTH_SIZABLE: usize = 1 << 1;
const NS_VIEW_HEIGHT_SIZABLE: usize = 1 << 4;

const HOST_VIEW_MARKER: &str = " <- native-page-layer";

// ---------------------------------------------------------------------------
// Objective-C helpers
// ---------------------------------------------------------------------------

/// Reads a native-endian pointer out of the first 8 bytes of `buf`, erroring
/// (instead of ever touching a null/garbage pointer) if that isn't possible.
fn read_pointer(buf: &[u8], what: &str) -> Result<*mut AnyObject> {
    if buf.len() < 8 {
        return Err(Error::new(
            Status::InvalidArg,
            format!("{what}: buffer must be at least 8 bytes, got {}", buf.len()),
        ));
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&buf[0..8]);
    let ptr_val = usize::from_ne_bytes(bytes);
    if ptr_val == 0 {
        return Err(Error::new(
            Status::InvalidArg,
            format!("{what}: pointer is null"),
        ));
    }
    Ok(ptr_val as *mut AnyObject)
}

/// Builds an autoreleased NSString. Caller does not own a reference to it;
/// treat it exactly like any other Cocoa autoreleased return value (valid
/// for the remainder of this call).
fn nsstring(s: &str) -> *mut AnyObject {
    let cstr = CString::new(s).expect("no interior NUL");
    unsafe { msg_send![class!(NSString), stringWithUTF8String: cstr.as_ptr()] }
}

/// Extracts a Rust `String` from an (autoreleased or borrowed) NSString.
unsafe fn string_from_nsstring(nsstr: *mut AnyObject) -> String {
    if nsstr.is_null() {
        return String::new();
    }
    let c_str_ptr: *const c_char = unsafe { msg_send![nsstr, UTF8String] };
    if c_str_ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(c_str_ptr) }
        .to_string_lossy()
        .into_owned()
}

/// Releases an object this code currently holds a +1 reference to. Used
/// right after handing a freshly alloc/init'd object to a container
/// (addSubview:/addSublayer:) which takes its own retain — the standard
/// manual-refcounting pattern of not holding two references to something
/// exactly one owner (the layer/view tree) is meant to track.
unsafe fn release(obj: *mut AnyObject) {
    let _: () = unsafe { msg_send![obj, release] };
}

/// Disables all five properties we mutate from implicitly animating. Belt
/// and suspenders with the CATransaction disableActions flag: this is what
/// actually stops a crossfade if some other code opens its own transaction
/// with actions enabled around one of our layers.
unsafe fn install_null_actions(layer: *mut AnyObject) {
    let dict: *mut AnyObject = unsafe { msg_send![class!(NSMutableDictionary), dictionary] };
    let null_obj: *mut AnyObject = unsafe { msg_send![class!(NSNull), null] };
    for key in ["contents", "bounds", "position", "hidden", "zPosition", "sublayers"] {
        let key_str = nsstring(key);
        let _: () = unsafe { msg_send![dict, setObject: null_obj, forKey: key_str] };
    }
    let _: () = unsafe { msg_send![layer, setActions: dict] };
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

struct LayerEntry {
    layer: *mut AnyObject,
    last_rect: Option<(f64, f64, f64, f64)>,
    last_hidden: Option<bool>,
    last_z: Option<f64>,
}

#[derive(Default)]
struct State {
    content_view: Option<*mut AnyObject>,
    host_view: Option<*mut AnyObject>,
    host_layer: Option<*mut AnyObject>,
    layers: HashMap<String, LayerEntry>,
    presents: u32,
    rect_updates: u32,
}

impl State {
    /// Creates the page's CALayer on first use, wired with the fixed
    /// per-page configuration from the spike brief. Idempotent.
    unsafe fn get_or_create_layer(&mut self, page_id: &str) -> Result<*mut AnyObject> {
        if let Some(entry) = self.layers.get(page_id) {
            return Ok(entry.layer);
        }
        let host_layer = self.host_layer.ok_or_else(|| {
            Error::new(Status::GenericFailure, "attach() must be called before present()/setRects()")
        })?;

        let layer: *mut AnyObject = unsafe { msg_send![class!(CALayer), alloc] };
        let layer: *mut AnyObject = unsafe { msg_send![layer, init] };

        let gravity_resize = nsstring("resize");
        let _: () = unsafe { msg_send![layer, setContentsGravity: gravity_resize] };
        let _: () = unsafe { msg_send![layer, setOpaque: Bool::new(true)] };
        let _: () = unsafe { msg_send![layer, setMasksToBounds: Bool::new(false)] };
        let _: () = unsafe { msg_send![layer, setHidden: Bool::new(true)] };
        // Default (linear) filters: don't touch minificationFilter /
        // magnificationFilter, whose default is already "linear" and not
        // "trilinear" (trilinear is the one that forces mipmap generation).
        unsafe { install_null_actions(layer) };

        let _: () = unsafe { msg_send![host_layer, addSublayer: layer] };
        // The sublayer array now owns this layer; give up our alloc/init
        // reference so there's exactly one owner (see `release` doc comment).
        unsafe { release(layer) };

        self.layers.insert(
            page_id.to_string(),
            LayerEntry {
                layer,
                last_rect: None,
                last_hidden: None,
                last_z: None,
            },
        );
        Ok(layer)
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

// ---------------------------------------------------------------------------
// Public N-API surface
// ---------------------------------------------------------------------------

#[napi]
pub fn attach(content_view_handle: Buffer, insert_index: u32) -> Result<Vec<String>> {
    let content_view = read_pointer(&content_view_handle, "attach(contentViewHandle)")?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();

        // Re-attaching: tear down whatever we had before so this call is
        // idempotent rather than leaking a second host view.
        if state.host_view.is_some() || !state.layers.is_empty() {
            unsafe { detach_impl(&mut state) };
        }

        unsafe {
            let view: *mut AnyObject = msg_send![class!(NSView), alloc];
            let view: *mut AnyObject = msg_send![view, init];

            let bounds: CGRect = msg_send![content_view, bounds];
            let _: () = msg_send![view, setFrame: bounds];
            let _: () = msg_send![
                view,
                setAutoresizingMask: (NS_VIEW_WIDTH_SIZABLE | NS_VIEW_HEIGHT_SIZABLE)
            ];
            let _: () = msg_send![view, setWantsLayer: Bool::new(true)];

            let host_layer: *mut AnyObject = msg_send![view, layer];
            if host_layer.is_null() {
                return Err(Error::new(
                    Status::GenericFailure,
                    "attach: NSView.layer was nil after setWantsLayer(YES)",
                ));
            }
            // Sublayer coordinates should be top-left-origin, matching the
            // rects setRects() is given. Flipping the *host* layer's own
            // geometry is what achieves that for its direct sublayers,
            // independent of whether Electron's own content view happens to
            // be flipped (sublayer frames are computed purely in the host
            // layer's own coordinate space).
            let _: () = msg_send![host_layer, setGeometryFlipped: Bool::new(true)];

            let subviews: *mut AnyObject = msg_send![content_view, subviews];
            let count: usize = msg_send![subviews, count];
            let clamped_index = (insert_index as usize).min(count);

            if clamped_index == 0 {
                if count == 0 {
                    let _: () = msg_send![
                        content_view,
                        addSubview: view,
                        positioned: NS_WINDOW_BELOW,
                        relativeTo: std::ptr::null_mut::<AnyObject>()
                    ];
                } else {
                    let sibling: *mut AnyObject = msg_send![subviews, objectAtIndex: 0usize];
                    let _: () = msg_send![
                        content_view,
                        addSubview: view,
                        positioned: NS_WINDOW_BELOW,
                        relativeTo: sibling
                    ];
                }
            } else {
                let sibling: *mut AnyObject =
                    msg_send![subviews, objectAtIndex: (clamped_index - 1)];
                let _: () = msg_send![
                    content_view,
                    addSubview: view,
                    positioned: NS_WINDOW_ABOVE,
                    relativeTo: sibling
                ];
            }
            // The content view's subview array now owns `view`; give up our
            // alloc/init reference (see `release` doc comment).
            release(view);

            state.content_view = Some(content_view);
            state.host_view = Some(view);
            state.host_layer = Some(host_layer);

            Ok(describe_subviews(content_view, view))
        }
    })
}

/// Bottom-to-top description of `content_view`'s subviews, one line each:
/// "<index> <ClassName> x,y,w,h hidden=<bool>", marking `host_view`.
unsafe fn describe_subviews(content_view: *mut AnyObject, host_view: *mut AnyObject) -> Vec<String> {
    let subviews: *mut AnyObject = unsafe { msg_send![content_view, subviews] };
    let count: usize = unsafe { msg_send![subviews, count] };
    let mut lines = Vec::with_capacity(count);
    for i in 0..count {
        let view: *mut AnyObject = unsafe { msg_send![subviews, objectAtIndex: i] };
        let class_obj: *mut AnyObject = unsafe { msg_send![view, class] };
        let class_name_obj: *mut AnyObject = unsafe { msg_send![class_obj, description] };
        let class_name = unsafe { string_from_nsstring(class_name_obj) };
        let frame: CGRect = unsafe { msg_send![view, frame] };
        let hidden: Bool = unsafe { msg_send![view, isHidden] };
        let marker = if view == host_view { HOST_VIEW_MARKER } else { "" };
        lines.push(format!(
            "{} {} {},{},{},{} hidden={}{}",
            i,
            class_name,
            frame.origin.x,
            frame.origin.y,
            frame.size.width,
            frame.size.height,
            hidden.as_bool(),
            marker
        ));
    }
    lines
}

#[napi]
pub fn present(page_id: String, io_surface: Buffer) -> Result<()> {
    let surface = read_pointer(&io_surface, "present(ioSurface)")?;

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        let layer = unsafe { state.get_or_create_layer(&page_id)? };

        unsafe {
            let _: () = msg_send![class!(CATransaction), begin];
            let _: () = msg_send![class!(CATransaction), setDisableActions: Bool::new(true)];
            let _: () = msg_send![layer, setContents: surface];
            let _: () = msg_send![class!(CATransaction), commit];
            // Push the new contents before the caller (the IOSurface pool
            // owner) can release/reuse the previous frame's surface.
            let _: () = msg_send![class!(CATransaction), flush];
        }

        state.presents += 1;
        Ok(())
    })
}

#[napi(js_name = "setRects")]
pub fn set_rects(page_ids: Vec<String>, rects: Vec<f64>) -> Result<()> {
    if page_ids.len() * 4 != rects.len() {
        return Err(Error::new(
            Status::InvalidArg,
            format!(
                "setRects: pageIds.length ({}) * 4 must equal rects.length ({})",
                page_ids.len(),
                rects.len()
            ),
        ));
    }

    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if state.host_layer.is_none() {
            return Err(Error::new(
                Status::GenericFailure,
                "setRects: attach() must be called first",
            ));
        }

        unsafe {
            // AppKit owns a layer-backed view's geometry and keeps it y-up
            // whatever `geometryFlipped` says, so top-left rects are mirrored
            // against the host's current height here.
            let host_bounds: CGRect = msg_send![state.host_layer.unwrap(), bounds];
            let host_height = host_bounds.size.height;

            let _: () = msg_send![class!(CATransaction), begin];
            let _: () = msg_send![class!(CATransaction), setDisableActions: Bool::new(true)];

            for (i, page_id) in page_ids.iter().enumerate() {
                let x = rects[i * 4];
                let w = rects[i * 4 + 2];
                let h = rects[i * 4 + 3];
                let y = host_height - rects[i * 4 + 1] - h;
                let hidden = w <= 0.0 || h <= 0.0;
                let z = i as f64;

                let layer = state.get_or_create_layer(page_id)?;
                let entry = state.layers.get_mut(page_id).expect("just created/looked up");

                let rect = (x, y, w, h);
                if entry.last_rect != Some(rect) {
                    let _: () = msg_send![layer, setFrame: CGRect::new(x, y, w, h)];
                    entry.last_rect = Some(rect);
                }
                if entry.last_hidden != Some(hidden) {
                    let _: () = msg_send![layer, setHidden: Bool::new(hidden)];
                    entry.last_hidden = Some(hidden);
                }
                if entry.last_z != Some(z) {
                    let _: () = msg_send![layer, setZPosition: z];
                    entry.last_z = Some(z);
                }
            }

            let _: () = msg_send![class!(CATransaction), commit];
        }

        state.rect_updates += 1;
        Ok(())
    })
}

#[napi]
pub fn remove(page_id: String) -> Result<()> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(entry) = state.layers.remove(&page_id) {
            unsafe {
                let _: () = msg_send![class!(CATransaction), begin];
                let _: () = msg_send![class!(CATransaction), setDisableActions: Bool::new(true)];
                let _: () = msg_send![entry.layer, setContents: std::ptr::null_mut::<AnyObject>()];
                let _: () = msg_send![entry.layer, removeFromSuperlayer];
                let _: () = msg_send![class!(CATransaction), commit];
            }
        }
        Ok(())
    })
}

#[napi(object)]
pub struct Stats {
    pub presents: u32,
    pub layers: u32,
    pub rect_updates: u32,
}

#[napi]
pub fn stats() -> Stats {
    STATE.with(|state| {
        let state = state.borrow();
        Stats {
            presents: state.presents,
            layers: state.layers.len() as u32,
            rect_updates: state.rect_updates,
        }
    })
}

/// Shared by `detach()` and `attach()`'s re-attach path. Assumes `state` is
/// already borrowed mutably by the caller.
unsafe fn detach_impl(state: &mut State) {
    for (_, entry) in state.layers.drain() {
        unsafe {
            let _: () = msg_send![entry.layer, setContents: std::ptr::null_mut::<AnyObject>()];
            let _: () = msg_send![entry.layer, removeFromSuperlayer];
        }
    }
    if let Some(host_view) = state.host_view.take() {
        unsafe {
            let _: () = msg_send![host_view, removeFromSuperview];
        }
    }
    state.host_layer = None;
    state.content_view = None;
}

#[napi]
pub fn detach() -> Result<()> {
    STATE.with(|state| {
        let mut state = state.borrow_mut();
        unsafe { detach_impl(&mut state) };
        Ok(())
    })
}
