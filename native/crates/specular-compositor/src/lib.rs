//! wgpu compositor for the Rust CEF spike.
//!
//! Draws every page's latest frame at its camera-projected rect over a dot
//! grid, in one render pass per window frame — the Rust analogue of
//! canvas-bg's `CanvasItemSurface` (CONTEXT.md, "Page textures"). The grid is
//! one procedural full-screen fill; pages are instanced quads with rounded
//! corners clipped in the fragment shader. Shapes ([`ShapeDraw`]) are
//! untextured SDF rounded rects drawn above every page in one instanced draw.
//!
//! Frame ingestion: [`Compositor::handle_page_event`] takes every
//! [`PageEvent`](specular_core::PageEvent) a source emits. GPU shared frames
//! are imported zero-copy (macOS: IOSurface -> `MTLTexture` -> wgpu-hal Metal
//! texture, cached per surface so Chromium's recycled surfaces import once)
//! and their surfaces are held until the GPU has finished sampling them;
//! the producer caps how many a page may hold
//! ([`MAX_OUTSTANDING_TEXTURES`](specular_core::MAX_OUTSTANDING_TEXTURES)).
//! CPU frames are uploaded with `Queue::write_texture` (dirty rects
//! only) and counted as non-representative in [`RenderStats`].

mod compositor;
mod draw_list;
mod error;
mod gpu;
mod gpu_types;
mod grid;
mod import;
mod import_cache;
mod instance_buffer;
mod instrumentation;
mod layers;
mod pipeline;
mod retire;
mod scene;
mod shape_list;
mod upload;

pub use compositor::Compositor;
pub use draw_list::PAGE_CORNER_RADIUS;
pub use error::{CompositorError, FrameImportError};
pub use gpu::GpuContext;
pub use instrumentation::{FrameObserver, FrameSample};
pub use scene::{DotGrid, PageDraw, RenderStats, SceneView, ShapeDraw, ShapeExtent};
