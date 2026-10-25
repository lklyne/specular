//! Pure, platform-free model for the Specular Rust CEF spike.
//!
//! Nothing in this crate touches a GPU, a window, or CEF. It defines the
//! contracts the other crates meet:
//!
//! - [`camera`] — canvas camera (pan/zoom) and world <-> screen projection,
//!   matching the Electron app's `applyViewportInputDelta` math.
//! - [`geometry`] — CSS / pixel / canvas-space size and rect types.
//! - [`page`] — [`PageId`], [`PageSpec`] and its validation: a live web item on the canvas.
//! - [`frame`] — [`PageFrame`]: what a page host paints (GPU shared surface or
//!   CPU BGRA bytes), plus popup layers.
//! - [`input`] — [`InputEvent`]: pointer / wheel / key / IME events forwarded
//!   into a page, in page-local CSS pixels.
//! - [`ledger`]: the resize ledger, an event log that is off unless the
//!   environment asks for it.
//! - [`source`] — the [`PageSource`] trait every page backend implements.
//! - [`text`]: how text is set, and the [`TextMeasure`](text::TextMeasure)
//!   the renderer implements for the editor.
//! - [`synthetic`] — [`SyntheticPageSource`], a CEF-free backend that paints
//!   animated CPU frames so the app and bench run anywhere.
//!
//! The canvas document is not here: it is `specular-doc`.

mod camera;
mod cursor;
mod frame;
mod geometry;
mod input;
pub mod ledger;
mod locator;
mod page;
mod source;
mod synthetic;
pub mod text;

pub use camera::{Camera, ViewportInputDelta};
pub use cursor::Cursor;
pub use frame::{
    CpuFrame, FrameEvent, FrameLayer, MAX_OUTSTANDING_TEXTURES, NativeSurface, PageFrame,
    PixelFormat, SharedTexture,
};
pub use geometry::{CssRect, CssSize, PixelRect, PixelSize, Point, Rect, Size};
pub use input::{
    EditingKey, ImeEvent, InputEvent, KeyEvent, KeyEventKind, Modifiers, PageEdit, PointerButton,
    PointerEvent, PointerEventKind, TouchEvent, TouchPhase, WheelEvent,
};
pub use locator::{
    LocatorBundle, LocatorCandidate, LocatorRect, LocatorResolution, resolve_locator,
};
pub use page::{PageId, PageSpec, validate_texture_scale, validate_viewport};
pub use source::{
    CapturedElement, DEVTOOLS_CLIENT_ID_BASE, DevtoolsSink, ElementPlace, InspectedNode,
    PageColorScheme, PageDevice, PageElement, PageEvent, PageNav, PageSafeArea, PageSource,
    PageSourceError, PageUserAgent, PointKind,
};
pub use synthetic::SyntheticPageSource;
