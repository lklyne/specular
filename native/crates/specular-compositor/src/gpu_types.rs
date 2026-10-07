//! Byte layouts shared with `shaders/canvas.wgsl`.

use bytemuck::{Pod, Zeroable};
use glam::Vec2;

use specular_core::Camera;

use crate::grid::GridMetrics;
use crate::scene::DotGrid;

/// The `Frame` uniform block. Field order and padding follow WGSL's uniform
/// layout rules; `FRAME_UNIFORMS_SIZE` pins the total.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct FrameUniforms {
    view_proj: [[f32; 4]; 4],
    viewport_px: [f32; 2],
    scale_factor: f32,
    zoom: f32,
    grid_origin: [f32; 2],
    grid_spacing: f32,
    dot_radius: f32,
    background: [f32; 4],
    dot: [f32; 4],
    encode_srgb: f32,
    padding: [f32; 3],
}

/// Size of [`FrameUniforms`] as WGSL sees it.
pub(crate) const FRAME_UNIFORMS_SIZE: u64 = 144;
const _: () = assert!(size_of::<FrameUniforms>() as u64 == FRAME_UNIFORMS_SIZE);

impl FrameUniforms {
    /// Uniforms for one frame. `viewport` is in logical pixels. `encode_srgb`
    /// is set when the target is a non-sRGB format, so the shaders encode
    /// linear colours themselves.
    pub(crate) fn new(
        camera: &Camera,
        viewport: Vec2,
        scale_factor: f32,
        grid: &DotGrid,
        metrics: &GridMetrics,
        encode_srgb: bool,
    ) -> Self {
        let [r, g, b, a] = grid.dot;
        Self {
            view_proj: camera.view_projection(viewport).to_cols_array_2d(),
            viewport_px: (viewport * scale_factor).to_array(),
            scale_factor,
            zoom: camera.zoom,
            grid_origin: metrics.origin.to_array(),
            grid_spacing: metrics.spacing,
            dot_radius: metrics.radius,
            background: grid.background,
            dot: [r, g, b, a * metrics.alpha],
            encode_srgb: if encode_srgb { 1.0 } else { 0.0 },
            padding: [0.0; 3],
        }
    }
}

/// One textured quad in canvas space: a page view, a popup or an image.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct QuadInstance {
    /// `x, y, width, height` in canvas units.
    pub(crate) rect: [f32; 4],
    /// The part of the texture to show: `u, v, width, height` as fractions.
    pub(crate) uv_rect: [f32; 4],
    /// Corner radius in canvas units (scaled by zoom in the shader).
    pub(crate) corner_radius: f32,
    /// Multiplies the premultiplied texel.
    pub(crate) opacity: f32,
    padding: [f32; 2],
}

impl QuadInstance {
    /// An opaque quad at `origin`/`size` showing the whole texture, with
    /// rounded corners.
    pub(crate) fn new(origin: Vec2, size: Vec2, corner_radius: f32) -> Self {
        Self {
            rect: [origin.x, origin.y, size.x, size.y],
            uv_rect: [0.0, 0.0, 1.0, 1.0],
            corner_radius,
            opacity: 1.0,
            padding: [0.0; 2],
        }
    }

    /// The same quad showing only `uv_rect` of the texture.
    pub(crate) fn with_uv_rect(self, uv_rect: [f32; 4]) -> Self {
        Self { uv_rect, ..self }
    }

    /// The same quad faded to `opacity`.
    pub(crate) fn with_opacity(self, opacity: f32) -> Self {
        Self { opacity, ..self }
    }

    /// Vertex attributes: `@location(0) rect`, `@location(1) uv_rect`,
    /// `@location(2)` corner radius and opacity.
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x2];
}

/// One untextured shape, resolved against the camera on the CPU so the
/// shader needs no per-shape branching.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct ShapeInstance {
    /// Centre of the rect, in canvas units.
    pub(crate) centre: [f32; 2],
    /// Half the rect's width and height, in logical pixels.
    pub(crate) half_size: [f32; 2],
    /// Fill colour, linear RGBA, straight alpha.
    pub(crate) fill: [f32; 4],
    /// Stroke colour, linear RGBA, straight alpha.
    pub(crate) stroke: [f32; 4],
    /// Corner radius in logical pixels.
    pub(crate) corner_radius: f32,
    /// Stroke width in logical pixels.
    pub(crate) stroke_width: f32,
    /// Where the stroke band starts, in logical pixels outwards from the
    /// edge: 0 puts the stroke outside, `-stroke_width` inside.
    pub(crate) stroke_offset: f32,
    /// [`Self::RECT`] or [`Self::ELLIPSE`].
    pub(crate) kind: f32,
}

impl ShapeInstance {
    /// `kind` of a rounded rect.
    pub(crate) const RECT: f32 = 0.0;
    /// `kind` of an ellipse inscribed in the rect.
    pub(crate) const ELLIPSE: f32 = 1.0;

    /// Vertex attributes: `@location(0)` centre and half size, `@location(1)`
    /// fill, `@location(2)` stroke, `@location(3)` radius, stroke width,
    /// stroke offset and kind.
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4];
}

/// One vertex of a tessellated path or polygon.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct MeshVertex {
    /// Position in logical screen pixels.
    pub(crate) position: [f32; 2],
    /// Colour, linear RGBA, straight alpha.
    pub(crate) color: [f32; 4],
}

impl MeshVertex {
    /// Vertex attributes: `@location(0) position`, `@location(1) color`.
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_instance_stride_is_sixteen_byte_aligned() {
        assert_eq!(size_of::<QuadInstance>() % 16, 0);
    }

    #[test]
    fn shape_instance_is_sixty_four_bytes() {
        assert_eq!(size_of::<ShapeInstance>(), 64);
    }
}
