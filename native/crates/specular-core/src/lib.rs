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
//! - [`source`] — the [`PageSource`] trait every page backend implements.
//! - [`synthetic`] — [`SyntheticPageSource`], a CEF-free backend that paints
//!   animated CPU frames so the app and bench run anywhere.
//!
//! The canvas document is not here: it is `specular-doc`.

pub mod camera;
pub mod frame;
pub mod geometry;
pub mod input;
pub mod page;
pub mod source;
pub mod synthetic;

pub use camera::{Camera, ViewportInputDelta};
pub use frame::{
    CpuFrame, FrameEvent, FrameLayer, MAX_OUTSTANDING_TEXTURES, NativeSurface, PageFrame,
    PixelFormat, SharedTexture,
};
pub use geometry::{CanvasRect, CssSize, PixelRect, PixelSize};
pub use input::{
    ImeEvent, InputEvent, KeyEvent, KeyEventKind, Modifiers, PointerButton, PointerEvent,
    PointerEventKind, WheelEvent,
};
pub use page::{PageId, PageSpec, validate_texture_scale, validate_viewport};
pub use source::{PageEvent, PageSource, PageSourceError};
pub use synthetic::SyntheticPageSource;
