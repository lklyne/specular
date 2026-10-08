//! Drawing a [`specular_scene::Scene`]: the display list where pages and
//! every other item share one z-order.
//!
//! A frame goes through four steps. [`place`] resolves items against the
//! camera and culls. [`batch`] groups the survivors into as few draws as the
//! z-order allows. [`build`] turns each batch into instances, triangles or
//! glyph quads. [`render`] encodes one 4x multisampled pass and submits it.
//! The first three are pure and tested without a GPU.

mod batch;
mod build;
mod color;
mod column;
mod dash;
mod emoji;
mod images;
mod mesh;
mod mips;
mod place;
mod raster_hold;
mod render;
mod shapes;
mod target;
mod text;
mod text_areas;
mod text_layout;
mod text_measure;
mod text_shape;

use std::collections::HashMap;

use glam::Vec2;
use specular_core::Camera;
use specular_scene::ImageId;

use self::build::DrawOp;
use self::images::ImageTexture;
use self::mesh::{Mesh, Mesher};
pub use self::mips::{ImageMips, ImageSpec};
use self::place::Placed;
use self::target::MultisampledTarget;
use self::text::TextSystem;
pub use self::text_measure::GlyphMeasure;
use crate::fonts::Fonts;
use crate::gpu_types::MeshVertex;
use crate::instance_buffer::InstanceBuffer;
use crate::scene::{DotGrid, RenderStats};

/// The camera and surface a [`Scene`](specular_scene::Scene) is drawn for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameView {
    /// Canvas camera, applied to canvas-space items.
    pub camera: Camera,
    /// Viewport size in logical pixels.
    pub viewport: Vec2,
    /// Physical pixels per logical pixel (window scale factor).
    pub scale_factor: f32,
    /// Background grid, drawn under every item.
    pub grid: DotGrid,
    /// Whether a zoom gesture is in flight. While it is, canvas text keeps
    /// the glyph size it was last rasterised at and is stretched to fit, so
    /// a zoom does not re-rasterise every glyph on every frame. Render one
    /// frame with this `false` when the gesture ends to sharpen the text.
    pub zooming: bool,
}

/// What [`Compositor::render_scene`](crate::Compositor::render_scene) drew.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SceneStats {
    /// The page and frame counters every render reports. `shapes_drawn`
    /// counts the scene's rects and ellipses.
    pub render: RenderStats,
    /// Items drawn, after culling.
    pub items_drawn: u32,
    /// Items left out because they were off screen, clipped away or fully
    /// transparent.
    pub items_culled: u32,
    /// Text runs left out because they were too small to read.
    pub text_runs_too_small: u32,
    /// Draw batches the items were grouped into.
    pub batches: u32,
    /// How many of those batches were text, each with its own glyph buffer.
    pub text_batches: u32,
}

/// State the scene pass keeps between frames.
#[derive(Debug)]
pub(crate) struct ScenePass {
    placed: Vec<Placed>,
    draws: Vec<DrawOp>,
    mesher: Mesher,
    mesh: Mesh,
    mesh_vertices: InstanceBuffer<MeshVertex>,
    mesh_indices: InstanceBuffer<u32>,
    /// Built on the first frame that shows text.
    text: Option<TextSystem>,
    /// The fonts the text system shapes with, and any measure handed out.
    fonts: Fonts,
    multisampled: Option<MultisampledTarget>,
    images: HashMap<ImageId, ImageTexture>,
}

impl ScenePass {
    /// Builds the text system if no frame has needed it yet.
    pub(crate) fn warm_text(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_format: wgpu::TextureFormat,
    ) {
        let fonts = &self.fonts;
        self.text
            .get_or_insert_with(|| TextSystem::new(device, queue, target_format, fonts.clone()));
    }

    /// The height of each owned column the latest frame drew.
    pub(crate) fn column_heights(&self) -> &[(specular_doc::EntityId, f32)] {
        (self.text.as_ref()).map_or(&[], TextSystem::column_heights)
    }

    /// A text measure on the fonts this pass draws with.
    pub(crate) fn text_measure(&self) -> GlyphMeasure {
        GlyphMeasure::sharing(self.fonts.clone())
    }

    pub(crate) fn new(device: &wgpu::Device) -> Self {
        Self {
            placed: Vec::new(),
            draws: Vec::new(),
            mesher: Mesher::default(),
            mesh: Mesh::new(),
            mesh_vertices: InstanceBuffer::new(device, "mesh-vertices"),
            mesh_indices: InstanceBuffer::with_usage(
                device,
                "mesh-indices",
                wgpu::BufferUsages::INDEX,
            ),
            text: None,
            fonts: Fonts::default(),
            multisampled: None,
            images: HashMap::new(),
        }
    }
}
