//! wgpu compositor for the Rust CEF spike.
//!
//! Draws every page's latest frame at its camera-projected rect over a dot
//! grid, in one render pass per window frame — the Rust analogue of
//! canvas-bg's `CanvasItemSurface` (CONTEXT.md, "Page textures").
//!
//! Frame ingestion: [`Compositor::handle_page_event`] takes every
//! [`PageEvent`](specular_core::PageEvent) a source emits. GPU shared frames
//! are imported zero-copy (macOS: IOSurface -> `MTLTexture` -> wgpu-hal Metal
//! texture); CPU frames are uploaded with `Queue::write_texture` and counted
//! as non-representative in [`RenderStats`].

mod compositor;
mod error;
mod gpu;

pub use compositor::{Compositor, DotGrid, PageDraw, RenderStats, SceneView};
pub use error::CompositorError;
pub use gpu::GpuContext;
