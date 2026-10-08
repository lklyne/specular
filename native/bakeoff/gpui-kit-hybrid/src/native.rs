//! AppKit glue: the canvas child view, its display link, a raw `NSEvent`
//! monitor, and synthesized input for the scripted runs.

use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use block2::RcBlock;
use objc2::runtime::{AnyClass, AnyObject, Bool, ClassBuilder, Sel};
use objc2::{class, msg_send, sel};
use objc2_foundation::{NSPoint, NSRect, NSRunLoopCommonModes, NSSize, NSString};

use crate::log;

pub type Id = *mut AnyObject;

/// When set, the canvas view answers `hitTest:` with nil, so AppKit sends
/// pointer events to GPUI's view underneath it.
pub static PASS_THROUGH: AtomicBool = AtomicBool::new(false);

extern "C-unwind" fn tick(_this: &AnyObject, _cmd: Sel, link: Id) {
    // SAFETY: `link` is the live CADisplayLink calling us.
    let target: f64 = unsafe { msg_send![link, targetTimestamp] };
    crate::canvas::display_link_fired(target);
}

extern "C-unwind" fn accepts_first_responder(_this: &AnyObject, _cmd: Sel) -> Bool {
    Bool::YES
}

extern "C-unwind" fn hit_test(this: &AnyObject, _cmd: Sel, point: NSPoint) -> Id {
    if PASS_THROUGH.load(Ordering::Relaxed) {
        return std::ptr::null_mut();
    }
    // SAFETY: forwarding the same message to NSView's implementation.
    unsafe { msg_send![super(this, class!(NSView)), hitTest: point] }
}

/// A native event arriving at the canvas view itself (the `above` layering).
extern "C-unwind" fn view_event(_this: &AnyObject, _cmd: Sel, event: Id) {
    log::line(format!("view   {}", describe_event(event)));
}

fn canvas_view_class() -> &'static AnyClass {
    static CLASS: OnceLock<&'static AnyClass> = OnceLock::new();
    CLASS.get_or_init(|| {
        let mut builder = ClassBuilder::new(c"SpikeCanvasView", class!(NSView))
            .expect("SpikeCanvasView is registered once");
        // SAFETY: each function's signature matches its selector's.
        unsafe {
            builder.add_method(sel!(spikeTick:), tick as extern "C-unwind" fn(_, _, _));
            builder.add_method(
                sel!(acceptsFirstResponder),
                accepts_first_responder as extern "C-unwind" fn(_, _) -> _,
            );
            builder.add_method(
                sel!(hitTest:),
                hit_test as extern "C-unwind" fn(_, _, _) -> _,
            );
            for selector in [
                sel!(mouseDown:),
                sel!(mouseUp:),
                sel!(mouseDragged:),
                sel!(rightMouseDown:),
                sel!(scrollWheel:),
                sel!(magnifyWithEvent:),
                sel!(keyDown:),
                sel!(keyUp:),
                sel!(flagsChanged:),
            ] {
                builder.add_method(selector, view_event as extern "C-unwind" fn(_, _, _));
            }
        }
        builder.register()
    })
}

/// Where the canvas view sits relative to GPUI's Metal view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Below,
    Above,
}

/// The views and layer of one canvas child. Copies share the same objects.
#[derive(Clone, Copy)]
pub struct NativeCanvas {
    pub view: Id,
    pub layer: Id,
    pub gpui_view: Id,
    pub window: Id,
    link: Id,
}

impl NativeCanvas {
    /// Creates a layer-hosting NSView with a CAMetalLayer and adds it to the
    /// window's content view, as a sibling of GPUI's view.
    pub fn new(gpui_view: Id, order: Order) -> Self {
        // SAFETY: main thread; `gpui_view` is GPUI's live NSView, and every
        // message below is a documented AppKit / QuartzCore call.
        unsafe {
            let content: Id = msg_send![gpui_view, superview];
            let window: Id = msg_send![gpui_view, window];
            let frame: NSRect = msg_send![content, bounds];
            let view: Id = msg_send![canvas_view_class(), alloc];
            let view: Id = msg_send![view, initWithFrame: frame];
            let layer: Id = msg_send![class!(CAMetalLayer), new];
            let _: () = msg_send![layer, setOpaque: true];
            // Layer-hosting: the layer is set before `wantsLayer`.
            let _: () = msg_send![view, setLayer: layer];
            let _: () = msg_send![view, setWantsLayer: true];
            // NSWindowAbove = 1, NSWindowBelow = -1.
            let place: isize = if order == Order::Above { 1 } else { -1 };
            let _: () =
                msg_send![content, addSubview: view, positioned: place, relativeTo: gpui_view];
            SYNTH_WINDOW.store(window, Ordering::Relaxed);
            // Floating and in front: macOS stops handing out drawables for a
            // covered window, which a measuring run would record as no frames.
            let _: () = msg_send![window, setLevel: 3isize];
            let _: () = msg_send![window, orderFrontRegardless];
            Self {
                view,
                layer,
                gpui_view,
                window,
                link: std::ptr::null_mut(),
            }
        }
    }

    /// Starts a display link on the view that calls the canvas each refresh.
    pub fn start_display_link(&mut self) {
        // SAFETY: `displayLinkWithTarget:selector:` (macOS 14) returns a link
        // bound to the view's display; the view implements `spikeTick:`.
        unsafe {
            let link: Id =
                msg_send![self.view, displayLinkWithTarget: self.view, selector: sel!(spikeTick:)];
            let _: Id = msg_send![link, retain];
            let run_loop: Id = msg_send![class!(NSRunLoop), mainRunLoop];
            let _: () = msg_send![link, addToRunLoop: run_loop, forMode: NSRunLoopCommonModes];
            self.link = link;
        }
    }

    /// Size of the content view in points.
    pub fn content_size(&self) -> (f64, f64) {
        // SAFETY: reading a live view's bounds.
        unsafe {
            let content: Id = msg_send![self.gpui_view, superview];
            let bounds: NSRect = msg_send![content, bounds];
            (bounds.size.width, bounds.size.height)
        }
    }

    /// Moves the canvas view to a rect given from the content view's top-left.
    pub fn set_frame_top_left(&self, x: f64, y: f64, width: f64, height: f64) {
        let (_, content_height) = self.content_size();
        let frame = NSRect::new(
            NSPoint::new(x, content_height - y - height),
            NSSize::new(width, height),
        );
        // SAFETY: setting a live view's frame on the main thread.
        unsafe {
            let _: () = msg_send![self.view, setFrame: frame];
        }
    }

    pub fn scale(&self) -> f64 {
        // SAFETY: reading a live window's backing scale.
        unsafe { msg_send![self.window, backingScaleFactor] }
    }

    pub fn set_contents_scale(&self, scale: f64) {
        // SAFETY: setting a property of our own layer.
        unsafe {
            let _: () = msg_send![self.layer, setContentsScale: scale];
        }
    }

    pub fn window_number(&self) -> isize {
        // SAFETY: reading a live window's number.
        unsafe { msg_send![self.window, windowNumber] }
    }

    /// Resizes the window's frame, keeping its top-left corner.
    pub fn set_window_content_size(&self, width: f64, height: f64) {
        // SAFETY: plain AppKit geometry calls on a live window.
        unsafe {
            let frame: NSRect = msg_send![self.window, frame];
            let content = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
            let wanted: NSRect = msg_send![self.window, frameRectForContentRect: content];
            let top = frame.origin.y + frame.size.height;
            let next = NSRect::new(
                NSPoint::new(frame.origin.x, top - wanted.size.height),
                wanted.size,
            );
            let _: () = msg_send![self.window, setFrame: next, display: true];
        }
    }

    pub fn close(&mut self) {
        // SAFETY: the link, view and layer are ours and still retained.
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

fn string_of(value: Id) -> String {
    if value.is_null() {
        return String::new();
    }
    // SAFETY: `value` is an NSString returned by AppKit.
    unsafe { (*(value as *const NSString)).to_string() }
}

/// One line saying what a raw `NSEvent` carries.
pub fn describe_event(event: Id) -> String {
    // SAFETY: `event` is a live NSEvent; each property read is valid for the
    // event types it is read for.
    unsafe {
        let kind: usize = msg_send![event, type];
        let flags: usize = msg_send![event, modifierFlags];
        let mods = flags & 0x00FF_0000;
        match kind {
            10 | 11 => {
                let code: u16 = msg_send![event, keyCode];
                let repeat: bool = msg_send![event, isARepeat];
                let chars: Id = msg_send![event, characters];
                let plain: Id = msg_send![event, charactersIgnoringModifiers];
                format!(
                    "{} keyCode={code} mods={mods:#x} repeat={repeat} chars={:?} unmodified={:?}",
                    if kind == 10 { "keyDown" } else { "keyUp" },
                    string_of(chars),
                    string_of(plain),
                )
            }
            12 => {
                let code: u16 = msg_send![event, keyCode];
                format!("flagsChanged keyCode={code} mods={mods:#x}")
            }
            22 => {
                let dx: f64 = msg_send![event, scrollingDeltaX];
                let dy: f64 = msg_send![event, scrollingDeltaY];
                let precise: bool = msg_send![event, hasPreciseScrollingDeltas];
                let phase: usize = msg_send![event, phase];
                let momentum: usize = msg_send![event, momentumPhase];
                let at: NSPoint = msg_send![event, locationInWindow];
                let window: isize = msg_send![event, windowNumber];
                format!(
                    "scrollWheel dx={dx} dy={dy} precise={precise} phase={phase} momentum={momentum} mods={mods:#x} at=({:.0},{:.0}) window={window}",
                    at.x, at.y
                )
            }
            30 => {
                let magnification: f64 = msg_send![event, magnification];
                let phase: usize = msg_send![event, phase];
                format!("magnify magnification={magnification} phase={phase}")
            }
            1..=6 => {
                let at: NSPoint = msg_send![event, locationInWindow];
                let clicks: isize = if kind == 5 || kind == 6 {
                    0
                } else {
                    msg_send![event, clickCount]
                };
                let name = [
                    "",
                    "mouseDown",
                    "mouseUp",
                    "rightDown",
                    "rightUp",
                    "moved",
                    "dragged",
                ][kind];
                format!(
                    "{name} at=({:.0},{:.0}) clicks={clicks} mods={mods:#x}",
                    at.x, at.y
                )
            }
            other => format!("event type={other}"),
        }
    }
}

/// Installs an app-local monitor that logs raw key, scroll and magnify
/// events before AppKit dispatches them to any view.
pub fn install_event_monitor() {
    let mask: u64 = (1 << 10) | (1 << 11) | (1 << 12) | (1 << 22) | (1 << 30) | (1 << 3) | (1 << 4);
    let block = RcBlock::new(|event: NonNull<AnyObject>| -> Id {
        let event = event.as_ptr();
        log::line(format!("monitor {}", describe_event(event)));
        if FORWARD_KEYS.load(Ordering::Relaxed) {
            forward_key(event);
        }
        if deliver_windowless(event) {
            return std::ptr::null_mut();
        }
        event
    });
    // SAFETY: the block takes and returns an NSEvent, as the API requires;
    // AppKit copies it. The returned token is deliberately kept for the
    // life of the process.
    unsafe {
        let _: Id = msg_send![class!(NSEvent), addLocalMonitorForEventsMatchingMask: mask, handler: &*block];
    }
}

/// When set, the monitor turns raw key events into page key events.
pub static FORWARD_KEYS: AtomicBool = AtomicBool::new(false);

/// Builds CEF's three key events from one raw `NSEvent`, which carries
/// everything they need: the virtual key code, the flags, the repeat bit and
/// the characters. Letters and digits only; the full table is in
/// `specular-app/src/translate.rs`.
fn forward_key(event: Id) {
    use specular_core::{InputEvent, KeyEvent, KeyEventKind, Modifiers};
    // SAFETY: `event` is a live NSEvent; these properties are read only for
    // key events.
    let (kind, code, flags, chars) = unsafe {
        let kind: usize = msg_send![event, type];
        if kind != 10 && kind != 11 {
            return;
        }
        let code: u16 = msg_send![event, keyCode];
        let flags: usize = msg_send![event, modifierFlags];
        let chars: Id = msg_send![event, characters];
        (kind, code, flags, string_of(chars))
    };
    let character = chars.chars().next();
    let modifiers = Modifiers {
        shift: flags & (1 << 17) != 0,
        control: flags & (1 << 18) != 0,
        alt: flags & (1 << 19) != 0,
        meta: flags & (1 << 20) != 0,
    };
    let windows_key_code = character
        .filter(char::is_ascii_alphanumeric)
        .map_or(0, |c| c.to_ascii_uppercase() as i32);
    // RawDown and Up carry the virtual key; Char carries the character as
    // its key code (`specular-interact/src/page_input.rs`).
    let key = |kind, windows_key_code, character| {
        InputEvent::Key(KeyEvent {
            kind,
            windows_key_code,
            native_key_code: i32::from(code),
            character,
            modifiers,
        })
    };
    crate::canvas::with(|canvas| {
        if kind == 10 {
            canvas.send_to_page(&key(KeyEventKind::RawDown, windows_key_code, None));
            if let Some(character) = character {
                canvas.send_to_page(&key(KeyEventKind::Char, character as i32, Some(character)));
            }
        } else {
            canvas.send_to_page(&key(KeyEventKind::Up, windows_key_code, None));
        }
    });
}

/// The window synthesized CoreGraphics events are meant for.
static SYNTH_WINDOW: std::sync::atomic::AtomicPtr<AnyObject> =
    std::sync::atomic::AtomicPtr::new(std::ptr::null_mut());

/// A synthesized CoreGraphics event reaches the process with no window, so
/// AppKit would drop it. This does AppKit's routing step by hand: it sends
/// the event to the view `hitTest:` finds under it, as `-[NSWindow
/// sendEvent:]` does for a hardware event. Returns whether it did.
fn deliver_windowless(event: Id) -> bool {
    let window = SYNTH_WINDOW.load(Ordering::Relaxed);
    // SAFETY: `event` is a live NSEvent and `window` our live window.
    unsafe {
        let number: isize = msg_send![event, windowNumber];
        if number != 0 || window.is_null() {
            return false;
        }
        let kind: usize = msg_send![event, type];
        let selector = match kind {
            3 => sel!(rightMouseDown:),
            4 => sel!(rightMouseUp:),
            22 => sel!(scrollWheel:),
            30 => sel!(magnifyWithEvent:),
            _ => return false,
        };
        let at: NSPoint = msg_send![event, locationInWindow];
        let content: Id = msg_send![window, contentView];
        let target: Id = msg_send![content, hitTest: at];
        if target.is_null() {
            return false;
        }
        let class: Id = msg_send![target, className];
        log::line(format!("route  to {}", string_of(class)));
        let _: Id = msg_send![target, performSelector: selector, withObject: event];
        true
    }
}

fn uptime() -> f64 {
    // SAFETY: NSProcessInfo is always available.
    unsafe {
        let info: Id = msg_send![class!(NSProcessInfo), processInfo];
        msg_send![info, systemUptime]
    }
}

fn post(event: Id) {
    if event.is_null() {
        log::line("synth  could not make the event".to_owned());
        return;
    }
    // SAFETY: posting a valid NSEvent to our own application's queue.
    unsafe {
        let app: Id = msg_send![class!(NSApplication), sharedApplication];
        let _: () = msg_send![app, postEvent: event, atStart: false];
    }
}

/// Synthesized input, posted to the application's own event queue so it
/// takes the same path as hardware events from `sendEvent:` on.
pub struct Synth<'a>(pub &'a NativeCanvas);

impl Synth<'_> {
    fn window_point(&self, x: f64, y: f64) -> NSPoint {
        let (_, height) = self.0.content_size();
        NSPoint::new(x, height - y)
    }

    /// `kind` is an `NSEventType` for a mouse button or move event; `x`, `y`
    /// are from the content view's top-left, in points.
    pub fn mouse(&self, kind: usize, x: f64, y: f64, clicks: isize) {
        let at = self.window_point(x, y);
        let number = self.0.window_number();
        let nil: Id = std::ptr::null_mut();
        // SAFETY: a documented NSEvent constructor with valid arguments.
        let event: Id = unsafe {
            msg_send![class!(NSEvent),
                mouseEventWithType: kind,
                location: at,
                modifierFlags: 0usize,
                timestamp: uptime(),
                windowNumber: number,
                context: nil,
                eventNumber: 0isize,
                clickCount: clicks,
                pressure: 1.0f32]
        };
        post(event);
    }

    pub fn click(&self, x: f64, y: f64) {
        self.mouse(5, x, y, 0);
        self.mouse(1, x, y, 1);
        self.mouse(2, x, y, 1);
    }

    /// Through CoreGraphics, because GPUI reads `buttonNumber`, which the
    /// NSEvent mouse constructor leaves at 0.
    pub fn right_click(&self, x: f64, y: f64) {
        self.mouse(5, x, y, 0);
        for kind in [3, 4] {
            // SAFETY: a documented CoreGraphics constructor; button 1 is right.
            let event = unsafe {
                CGEventCreateMouseEvent(std::ptr::null(), kind, self.cg_location(x, y), 1)
            };
            // SAFETY: field 1 is the click count of a mouse event we own.
            unsafe { CGEventSetIntegerValueField(event, 1, 1) };
            self.post_cg(event, x, y);
        }
    }

    /// A key event with a real macOS virtual key code.
    pub fn key(&self, down: bool, code: u16, flags: usize, chars: &str, plain: &str, repeat: bool) {
        let kind: usize = if down { 10 } else { 11 };
        let chars = NSString::from_str(chars);
        let plain = NSString::from_str(plain);
        let nil: Id = std::ptr::null_mut();
        // SAFETY: a documented NSEvent constructor with valid arguments.
        let event: Id = unsafe {
            msg_send![class!(NSEvent),
                keyEventWithType: kind,
                location: NSPoint::new(0.0, 0.0),
                modifierFlags: flags,
                timestamp: uptime(),
                windowNumber: self.0.window_number(),
                context: nil,
                characters: &*chars,
                charactersIgnoringModifiers: &*plain,
                isARepeat: repeat,
                keyCode: code]
        };
        post(event);
    }

    /// Where to put a CoreGraphics event. An event posted to our own
    /// process arrives with no window, so its `locationInWindow` is the
    /// screen point; this makes that equal the wanted window point, and the
    /// monitor then hands the event to the view under it.
    fn cg_location(&self, x: f64, y: f64) -> NSPoint {
        // SAFETY: plain AppKit geometry reads on live objects.
        unsafe {
            let screens: Id = msg_send![class!(NSScreen), screens];
            let first: Id = msg_send![screens, firstObject];
            let screen: NSRect = msg_send![first, frame];
            let (_, height) = self.0.content_size();
            NSPoint::new(x, screen.size.height - (height - y))
        }
    }

    /// Posts a CoreGraphics event to this process through the window
    /// server's per-process queue, so AppKit routes it by location like a
    /// hardware event.
    fn post_cg(&self, event: *mut c_void, x: f64, y: f64) {
        // SAFETY: `event` is a CGEvent we created and release here.
        unsafe {
            CGEventSetLocation(event, self.cg_location(x, y));
            CGEventPostToPid(std::process::id() as i32, event);
            CFRelease(event);
        }
    }

    /// A pixel-precise scroll, as a trackpad sends.
    pub fn scroll(&self, x: f64, y: f64, dx: i32, dy: i32) {
        // SAFETY: a documented CoreGraphics constructor; unit 0 is pixels.
        let event = unsafe { CGEventCreateScrollWheelEvent2(std::ptr::null(), 0, 2, dy, dx, 0) };
        self.post_cg(event, x, y);
    }

    /// A trackpad magnify gesture step. CoreGraphics has no public
    /// constructor for it, so this sets the gesture fields by number.
    pub fn magnify(&self, x: f64, y: f64, amount: f64, phase: i64) {
        // SAFETY: a blank CGEvent retyped as a gesture event; the field
        // numbers are the ones AppKit reads for NSEventTypeMagnify.
        unsafe {
            let event = CGEventCreate(std::ptr::null());
            CGEventSetType(event, 29);
            CGEventSetIntegerValueField(event, 110, 8);
            CGEventSetDoubleValueField(event, 113, amount);
            CGEventSetIntegerValueField(event, 132, phase);
            self.post_cg(event, x, y);
        }
    }

    /// Drives GPUI's `NSTextInputClient` directly, as an input method does.
    pub fn set_marked_text(&self, text: &str, caret: usize) {
        let text = NSString::from_str(text);
        let selected = NSRangeRaw {
            location: caret,
            length: 0,
        };
        let replace = NSRangeRaw {
            location: usize::MAX >> 1,
            length: 0,
        };
        // SAFETY: GPUIView implements NSTextInputClient with this signature.
        unsafe {
            let _: () = msg_send![self.0.gpui_view,
                setMarkedText: &*text, selectedRange: selected, replacementRange: replace];
        }
    }

    pub fn insert_text(&self, text: &str) {
        let text = NSString::from_str(text);
        let replace = NSRangeRaw {
            location: usize::MAX >> 1,
            length: 0,
        };
        // SAFETY: as above.
        unsafe {
            let _: () = msg_send![self.0.gpui_view, insertText: &*text, replacementRange: replace];
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct NSRangeRaw {
    location: usize,
    length: usize,
}

// SAFETY: the layout and encoding are NSRange's.
unsafe impl objc2::Encode for NSRangeRaw {
    const ENCODING: objc2::Encoding =
        objc2::Encoding::Struct("NSRange", &[usize::ENCODING, usize::ENCODING]);
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreate(source: *const c_void) -> *mut c_void;
    fn CGEventCreateScrollWheelEvent2(
        source: *const c_void,
        units: u32,
        wheel_count: u32,
        wheel1: i32,
        wheel2: i32,
        wheel3: i32,
    ) -> *mut c_void;
    fn CGEventCreateMouseEvent(
        source: *const c_void,
        kind: u32,
        location: NSPoint,
        button: u32,
    ) -> *mut c_void;
    fn CGEventPostToPid(pid: i32, event: *mut c_void);
    fn CGEventSetType(event: *mut c_void, kind: u32);
    fn CGEventSetLocation(event: *mut c_void, location: NSPoint);
    fn CGEventSetIntegerValueField(event: *mut c_void, field: u32, value: i64);
    fn CGEventSetDoubleValueField(event: *mut c_void, field: u32, value: f64);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: *mut c_void);
}
