//! The wgpu compositor.
//!
//! [`Compositor::render_scene`] draws a [`specular_scene::Scene`] over a dot
//! grid in one 4x multisampled pass per window frame. Pages and every other
//! item share one z-order: page frames are textured quads with rounded
//! corners, rects and ellipses are distance fields, text is glyphon, and
//! polygons and paths are tessellated by lyon (ADR 0039).
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
mod scene_pass;
mod upload;

pub use compositor::Compositor;
pub use error::{CompositorError, FrameImportError};
pub use gpu::GpuContext;
pub use instrumentation::{FrameObserver, FrameSample};
pub use scene::{DotGrid, RenderStats};
pub use scene_pass::{FrameView, ImageMips, ImageSpec, SceneStats};
