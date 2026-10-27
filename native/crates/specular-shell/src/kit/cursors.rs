//! The cursors `AppKit` has no stock image for: the app's own arrow and the
//! fingertip over a page on a touch device.
//!
//! GPUI names a cursor by a style and hands `AppKit` the stock cursor for it
//! in one place, its view's `resetCursorRects`. That call is caught here,
//! and while the pointer is over the canvas the stock arrow is swapped for
//! the arrow of the [`CursorSet`] in use. Every other stock cursor goes
//! through as it is, and so does the arrow over the panels around the
//! canvas.
#![expect(
    clippy::multiple_unsafe_ops_per_block,
    reason = "AppKit calls come in runs on the same live objects; one SAFETY note covers a run"
)]

use std::cell::Cell;
use std::ffi::c_void;

use objc2::runtime::{AnyObject, Imp, Sel};
use objc2::{class, msg_send, sel};
use objc2_foundation::{NSPoint, NSRect};
use specular_core::Cursor;
use specular_scene::arrow_cursor_svg;

use crate::native::Id;

/// Which image the arrow is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CursorSet {
    /// The app's own arrow: the select tool's glyph, black with a white
    /// line.
    #[default]
    Specular,
    /// The system's arrow, over a page that is pointed at as a browser
    /// points at it.
    System,
    /// A fingertip, over a page on a touch device.
    Touch,
}

impl CursorSet {
    /// The set `cursor` is drawn from.
    pub(crate) const fn of(cursor: Cursor) -> Self {
        match cursor {
            Cursor::Touch => Self::Touch,
            Cursor::PageArrow => Self::System,
            Cursor::Default
            | Cursor::Crosshair
            | Cursor::Grab
            | Cursor::Grabbing
            | Cursor::Move
            | Cursor::Text
            | Cursor::Pointer
            | Cursor::ResizeNwse
            | Cursor::ResizeNesw
            | Cursor::ResizeEw
            | Cursor::ResizeNs
            | Cursor::ResizeColumn
            | Cursor::ResizeRow
            | Cursor::VerticalText
            | Cursor::NotAllowed
            | Cursor::Alias
            | Cursor::Copy
            | Cursor::ContextMenu => Self::Specular,
        }
    }
}

/// The side of the select tool's glyph, in points.
const ARROW_SIZE: f64 = 20.0;
/// The tip of the glyph's arrow.
const ARROW_TIP: (f64, f64) = (3.0, 3.0);
/// The fingertip's side, in points: the circle of Chrome's device mode.
const TOUCH_SIZE: f64 = 24.0;
const TOUCH_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" \
    width=\"24\" height=\"24\"><circle cx=\"12\" cy=\"12\" r=\"10.5\" fill=\"#808080\" \
    fill-opacity=\"0.45\" stroke=\"#ffffff\" stroke-opacity=\"0.7\" stroke-width=\"1\"/>\
    <circle cx=\"12\" cy=\"12\" r=\"9.5\" fill=\"none\" stroke=\"#000000\" \
    stroke-opacity=\"0.35\" stroke-width=\"1\"/></svg>";

thread_local! {
    static SET: Cell<CursorSet> = const { Cell::new(CursorSet::Specular) };
    /// Whether the pointer is the canvas's: over it, or dragged off it.
    static OVER_CANVAS: Cell<bool> = const { Cell::new(false) };
    /// GPUI's view, once its cursor rects are caught.
    static VIEW: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
    static ARROW: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
    static TOUCH: Cell<Id> = const { Cell::new(std::ptr::null_mut()) };
}

/// An `NSCursor` of the SVG document `svg`, kept for the life of the app.
/// Nil when `AppKit` cannot read it, and the stock arrow then stays.
fn cursor_of(svg: &str, hot_spot: (f64, f64)) -> Id {
    // SAFETY: main thread. `NSData` copies the bytes, `NSImage` reads an
    // SVG document from data, and the cursor returned by `alloc`/`init` is
    // owned by the caller, which never releases it.
    unsafe {
        let data: Id = msg_send![
            class!(NSData),
            dataWithBytes: svg.as_ptr().cast::<c_void>(),
            length: svg.len()
        ];
        let image: Id = msg_send![class!(NSImage), alloc];
        let image: Id = msg_send![image, initWithData: data];
        if image.is_null() {
            return std::ptr::null_mut();
        }
        let cursor: Id = msg_send![class!(NSCursor), alloc];
        msg_send![
            cursor,
            initWithImage: image,
            hotSpot: NSPoint::new(hot_spot.0, hot_spot.1)
        ]
    }
}

/// GPUI's `addCursorRect:cursor:` on its own view, with the stock arrow
/// swapped for the set's.
extern "C-unwind" fn add_cursor_rect(this: &AnyObject, _cmd: Sel, rect: NSRect, cursor: Id) {
    // SAFETY: `this` is GPUI's view, whose superclass chain reaches
    // `NSView`, which implements `addCursorRect:cursor:` with this
    // signature. `arrowCursor` returns the shared stock arrow.
    unsafe {
        let stock: Id = msg_send![class!(NSCursor), arrowCursor];
        let ours = match SET.get() {
            _ if cursor != stock || !OVER_CANVAS.get() => std::ptr::null_mut(),
            CursorSet::Specular => ARROW.get(),
            CursorSet::Touch => TOUCH.get(),
            CursorSet::System => std::ptr::null_mut(),
        };
        let cursor = if ours.is_null() { cursor } else { ours };
        let _: () = msg_send![super(this, class!(NSView)), addCursorRect: rect, cursor: cursor];
    }
}

/// Catches the cursor rects of `gpui_view`, GPUI's view. Without it every
/// arrow is the system's.
pub(crate) fn install(gpui_view: Id) {
    let glyph = arrow_cursor_svg();
    ARROW.set(cursor_of(&glyph, ARROW_TIP));
    TOUCH.set(cursor_of(TOUCH_SVG, (TOUCH_SIZE / 2.0, TOUCH_SIZE / 2.0)));
    debug_assert!(glyph.contains(&format!("width=\"{ARROW_SIZE}\"")));
    let handler: extern "C-unwind" fn(&AnyObject, Sel, NSRect, Id) = add_cursor_rect;
    // SAFETY: main thread, and `gpui_view` is GPUI's live view. The method
    // is added to its class, which does not define `addCursorRect:cursor:`
    // itself, so `NSView`'s stays the one `super` reaches. The type string
    // is `void (id, SEL, NSRect, id)`, the handler's own signature.
    let added = unsafe {
        let class = objc2::ffi::object_getClass(gpui_view.cast_const());
        let imp =
            std::mem::transmute::<extern "C-unwind" fn(&AnyObject, Sel, NSRect, Id), Imp>(handler);
        objc2::ffi::class_addMethod(
            class.cast_mut(),
            sel!(addCursorRect:cursor:),
            imp,
            c"v@:{CGRect={CGPoint=dd}{CGSize=dd}}@".as_ptr(),
        )
    };
    if added.as_bool() {
        VIEW.set(gpui_view);
        refresh();
    } else {
        tracing::warn!(
            pin = %crate::pins::describe(),
            "GPUI's view sets its own cursor rects; the app's cursors are the system's"
        );
    }
}

/// Has `AppKit` ask GPUI's view for its cursor rects again.
fn refresh() {
    let view = VIEW.get();
    if view.is_null() {
        return;
    }
    // SAFETY: main thread, and GPUI's view outlives the window's canvas.
    unsafe {
        let window: Id = msg_send![view, window];
        let _: () = msg_send![window, invalidateCursorRectsForView: view];
    }
}

/// Draws the arrow from `set` from now on.
pub(crate) fn use_set(set: CursorSet) {
    if SET.replace(set) != set {
        refresh();
    }
}

/// Says whether the pointer is the canvas's. Off it, the arrow is the
/// system's.
pub(crate) fn over_canvas(over: bool) {
    if OVER_CANVAS.replace(over) != over {
        refresh();
    }
}
