//! `AppKit` glue: the canvas view under GPUI's, its display link, and the few
//! window calls GPUI has no public way to make from outside a GPUI update.
//!
//! ```text
//! NSWindow (GPUI's)
//! └─ contentView
//!    ├─ SpecularCanvasView   ours: hosts a CAMetalLayer, fills the window
//!    └─ GPUIView             GPUI's Metal layer, not opaque, drawn above
//! ```
//!
//! The canvas view never moves and never takes an event. GPUI leaves the
//! canvas slot unpainted, and that hole is where the canvas shows.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "AppKit calls come in runs on the same live objects; one SAFETY note covers a run"
)]

use std::sync::OnceLock;

use anyhow::{Context as _, bail};
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::{class, msg_send, sel};
use objc2_foundation::{NSPoint, NSRect, NSRunLoopCommonModes, NSSize, NSString};
use specular_core::ledger::{self, Entry};

/// An Objective-C object.
pub(crate) type Id = *mut AnyObject;

/// `NSViewWidthSizable | NSViewHeightSizable`.
const FILLS_SUPERVIEW: usize = 2 | 16;
/// `NSWindowBelow`.
const BELOW: isize = -1;
/// `NSFloatingWindowLevel`.
const FLOATING: isize = 3;

extern "C-unwind" fn tick(_this: &AnyObject, _cmd: Sel, _link: Id) {
    crate::canvas::on_display_link();
}

/// The canvas view is never under the pointer as far as `AppKit` knows, so
/// every event goes to GPUI's view and the canvas slot element.
extern "C-unwind" fn hit_test(_this: &AnyObject, _cmd: Sel, _point: NSPoint) -> Id {
    std::ptr::null_mut()
}

extern "C-unwind" fn is_opaque(_this: &AnyObject, _cmd: Sel) -> Bool {
    Bool::YES
}

/// The three overrides below only write to the resize ledger and hand on
/// to `NSView`.
extern "C-unwind" fn set_frame_size(this: &AnyObject, _cmd: Sel, size: NSSize) {
    ledger::record(Entry::WinSize(size.width as f32, size.height as f32));
    // SAFETY: `this` is a `SpecularCanvasView`, whose superclass is `NSView`,
    // which implements `setFrameSize:` with this signature.
    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), setFrameSize: size];
    }
}

extern "C-unwind" fn will_start_live_resize(this: &AnyObject, _cmd: Sel) {
    ledger::record(Entry::LiveBegin);
    // SAFETY: as in `set_frame_size`, for `viewWillStartLiveResize`.
    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), viewWillStartLiveResize];
    }
}

extern "C-unwind" fn did_end_live_resize(this: &AnyObject, _cmd: Sel) {
    ledger::record(Entry::LiveEnd);
    // SAFETY: as in `set_frame_size`, for `viewDidEndLiveResize`.
    unsafe {
        let _: () = msg_send![super(this, class!(NSView)), viewDidEndLiveResize];
    }
}

fn canvas_view_class() -> anyhow::Result<&'static AnyClass> {
    static CLASS: OnceLock<Option<&'static AnyClass>> = OnceLock::new();
    CLASS
        .get_or_init(|| {
            let mut builder = ClassBuilder::new(c"SpecularCanvasView", class!(NSView))?;
            // SAFETY: each function's signature matches its selector's:
            // `void (id, SEL, id)`, `NSView * (id, SEL, NSPoint)` and
            // `BOOL (id, SEL)`.
            unsafe {
                builder.add_method(sel!(specularTick:), tick as extern "C-unwind" fn(_, _, _));
                builder.add_method(
                    sel!(hitTest:),
                    hit_test as extern "C-unwind" fn(_, _, _) -> _,
                );
                builder.add_method(sel!(isOpaque), is_opaque as extern "C-unwind" fn(_, _) -> _);
            }
            if ledger::enabled() {
                // SAFETY: `void (id, SEL, NSSize)` and `void (id, SEL)`,
                // as `NSView` declares the three.
                unsafe {
                    builder.add_method(
                        sel!(setFrameSize:),
                        set_frame_size as extern "C-unwind" fn(_, _, _),
                    );
                    builder.add_method(
                        sel!(viewWillStartLiveResize),
                        will_start_live_resize as extern "C-unwind" fn(_, _),
                    );
                    builder.add_method(
                        sel!(viewDidEndLiveResize),
                        did_end_live_resize as extern "C-unwind" fn(_, _),
                    );
                }
            }
            Some(builder.register())
        })
        .context("the SpecularCanvasView class is already registered by something else")
}

/// The text of an `NSString`, or nothing for nil.
pub(crate) fn string_of(value: Id) -> String {
    if value.is_null() {
        return String::new();
    }
    // SAFETY: the callers pass an `NSString` AppKit returned, which lives
    // at least until the current autorelease pool drains.
    unsafe { (*value.cast::<NSString>()).to_string() }
}

/// Shows the standard About panel.
pub(crate) fn show_about() {
    let nil: Id = std::ptr::null_mut();
    // SAFETY: the shared application exists while GPUI runs, and the panel
    // takes a nil sender.
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, orderFrontStandardAboutPanel: nil];
    }
}

/// Has `AppKit` draw its own parts of the app, the traffic lights and the
/// sheets among them, dark or light, or as the system is for `None`.
pub(crate) fn set_app_appearance(dark: Option<bool>) {
    let name = dark.map(|dark| {
        NSString::from_str(if dark {
            "NSAppearanceNameDarkAqua"
        } else {
            "NSAppearanceNameAqua"
        })
    });
    // SAFETY: the shared application exists while GPUI runs, both names
    // are appearances AppKit has, and a nil appearance is the system's.
    unsafe {
        let appearance: Id = match &name {
            Some(name) => msg_send![class!(NSAppearance), appearanceNamed: &**name],
            None => std::ptr::null_mut(),
        };
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, setAppearance: appearance];
    }
}

/// Whether the system is dark, whatever the app is set to draw as.
pub(crate) fn system_is_dark() -> bool {
    let key = NSString::from_str("AppleInterfaceStyle");
    // SAFETY: the standard defaults always exist, and `stringForKey:`
    // returns an `NSString` or nil.
    let style: Id = unsafe {
        let defaults: Id = msg_send![class!(NSUserDefaults), standardUserDefaults];
        msg_send![defaults, stringForKey: &*key]
    };
    string_of(style) == "Dark"
}

fn class_name(object: Id) -> String {
    if object.is_null() {
        return "nil".to_owned();
    }
    // SAFETY: `object` is a live Objective-C object, and `className`
    // returns an `NSString`.
    string_of(unsafe { msg_send![object, className] })
}

/// The views and layer of the canvas. Copies share the same objects.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NativeCanvas {
    view: Id,
    layer: Id,
    window: Id,
    link: Id,
}

impl NativeCanvas {
    /// Puts a layer-hosting view with a `CAMetalLayer` under `gpui_view`,
    /// filling the window's content view.
    ///
    /// This is where the three facts about GPUI's macOS backend that no API
    /// promises are checked (ADR 0040, Risks). Each failure names the fact,
    /// so a bump of the pin that breaks one says which on its first launch.
    pub(crate) fn install(gpui_view: Id) -> anyhow::Result<Self> {
        let pin = crate::pins::describe();
        if gpui_view.is_null() {
            bail!("GPUI's window handle carries no NSView ({pin})");
        }
        // SAFETY: main thread, inside a GPUI window update. `gpui_view` is
        // the live `NSView` GPUI's window handle returned, and every message
        // is a documented AppKit or QuartzCore one.
        unsafe {
            let window: Id = msg_send![gpui_view, window];
            let content: Id = msg_send![window, contentView];
            let parent: Id = msg_send![gpui_view, superview];
            // Fact 1: GPUI's view is a subview of the content view, so a
            // sibling can go under it.
            if window.is_null() || content.is_null() || parent != content {
                bail!(
                    "GPUI's view is no longer a direct subview of its window's content view \
                     (its superview is a {}), so the canvas cannot be put under it ({pin})",
                    class_name(parent)
                );
            }
            // Fact 2: the view `window_handle` returns is the one GPUI
            // draws into, with a Metal layer of its own.
            let gpui_layer: Id = msg_send![gpui_view, layer];
            let is_metal: bool = if gpui_layer.is_null() {
                false
            } else {
                msg_send![gpui_layer, isKindOfClass: class!(CAMetalLayer)]
            };
            if !is_metal {
                bail!(
                    "the view from GPUI's window handle (a {}) is not the Metal view it draws \
                     into: its layer is a {} ({pin})",
                    class_name(gpui_view),
                    class_name(gpui_layer)
                );
            }
            // Fact 3: a transparent window makes that layer non-opaque. An
            // opaque one would hide the canvas under it.
            let opaque: bool = msg_send![gpui_layer, isOpaque];
            if opaque {
                bail!(
                    "GPUI's Metal layer is opaque in a window opened with a transparent \
                     background, so it would cover the canvas ({pin})"
                );
            }

            let frame: NSRect = msg_send![content, bounds];
            let view: Id = msg_send![canvas_view_class()?, alloc];
            let view: Id = msg_send![view, initWithFrame: frame];
            let layer: Id = msg_send![class!(CAMetalLayer), new];
            let _: () = msg_send![layer, setOpaque: true];
            // Layer-hosting: the layer is set before `wantsLayer`.
            let _: () = msg_send![view, setLayer: layer];
            let _: () = msg_send![view, setWantsLayer: true];
            let _: () = msg_send![view, setAutoresizingMask: FILLS_SUPERVIEW];
            let _: () =
                msg_send![content, addSubview: view, positioned: BELOW, relativeTo: gpui_view];
            Ok(Self {
                view,
                layer,
                window,
                link: std::ptr::null_mut(),
            })
        }
    }

    /// The `CAMetalLayer` to make a surface on.
    pub(crate) fn layer(&self) -> Id {
        self.layer
    }

    /// Starts a display link on the view's own display. Each refresh calls
    /// [`crate::canvas::on_display_link`] on the main thread.
    pub(crate) fn start_display_link(&mut self) {
        // SAFETY: `displayLinkWithTarget:selector:` (macOS 14) returns a
        // link bound to the view's display, and the view implements
        // `specularTick:`. The link is retained until `close`.
        unsafe {
            let link: Id = msg_send![
                self.view,
                displayLinkWithTarget: self.view,
                selector: sel!(specularTick:)
            ];
            if link.is_null() {
                return;
            }
            let _: Id = msg_send![link, retain];
            let run_loop: Id = msg_send![class!(NSRunLoop), mainRunLoop];
            let _: () = msg_send![link, addToRunLoop: run_loop, forMode: NSRunLoopCommonModes];
            self.link = link;
        }
    }

    /// The size of the content view, and so of the canvas view, in points.
    pub(crate) fn content_size(&self) -> (f64, f64) {
        // SAFETY: reading the bounds of our own live view.
        let bounds: NSRect = unsafe { msg_send![self.view, bounds] };
        (bounds.size.width, bounds.size.height)
    }

    /// Device pixels per point on the window's display.
    pub(crate) fn scale(&self) -> f64 {
        // SAFETY: reading a live window's backing scale.
        unsafe { msg_send![self.window, backingScaleFactor] }
    }

    /// Tells the layer the scale its drawables are in.
    pub(crate) fn set_contents_scale(&self, scale: f64) {
        // SAFETY: setting a property of our own layer.
        unsafe {
            let _: () = msg_send![self.layer, setContentsScale: scale];
        }
    }

    /// The window server's number for the window, which `screencapture -l`
    /// takes.
    pub(crate) fn window_number(&self) -> isize {
        // SAFETY: reading a live window's number.
        unsafe { msg_send![self.window, windowNumber] }
    }

    /// Sets the title and the close button's unsaved dot.
    pub(crate) fn set_title(&self, title: &str, unsaved: bool) {
        let title = NSString::from_str(title);
        // SAFETY: documented setters on a live window, on the main thread.
        unsafe {
            let _: () = msg_send![self.window, setTitle: &*title];
            let _: () = msg_send![self.window, setDocumentEdited: unsaved];
        }
    }

    /// Keeps the window above others and brings it forward. macOS hands a
    /// covered window no drawables, which a measuring or capturing run
    /// would record as no frames.
    pub(crate) fn float(&self) {
        // SAFETY: documented calls on a live window, on the main thread.
        unsafe {
            let _: () = msg_send![self.window, setLevel: FLOATING];
            let _: () = msg_send![self.window, orderFrontRegardless];
        }
    }

    /// Stops the display link firing, or lets it fire again. A canvas at
    /// rest has nothing for a refresh to do.
    pub(crate) fn set_link_paused(&self, paused: bool) {
        if self.link.is_null() {
            return;
        }
        // SAFETY: the link is ours and retained until `close`.
        unsafe {
            let _: () = msg_send![self.link, setPaused: paused];
        }
    }

    /// Stops the display link and takes the view out of the window.
    pub(crate) fn close(&mut self) {
        // SAFETY: the link and the view are ours and still retained.
        unsafe {
            if !self.link.is_null() {
                let _: () = msg_send![self.link, invalidate];
                let _: () = msg_send![self.link, release];
                self.link = std::ptr::null_mut();
            }
            let _: () = msg_send![self.view, removeFromSuperview];
        }
    }
}
