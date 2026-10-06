//! Byte layouts shared with `shaders/canvas.wgsl`.

use bytemuck::{Pod, Zeroable};
use glam::Vec2;

use crate::grid::GridMetrics;
use crate::scene::SceneView;

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
    /// Uniforms for `scene`. `encode_srgb` is set when the target is a
    /// non-sRGB format, so the shader encodes the linear grid colours itself.
    pub(crate) fn new(scene: &SceneView<'_>, grid: &GridMetrics, encode_srgb: bool) -> Self {
        let [r, g, b, a] = scene.grid.dot;
        let viewport_px = scene.viewport * scene.scale_factor;
        Self {
            view_proj: scene
                .camera
                .view_projection(scene.viewport)
                .to_cols_array_2d(),
            viewport_px: viewport_px.to_array(),
            scale_factor: scene.scale_factor,
            zoom: scene.camera.zoom,
            grid_origin: grid.origin.to_array(),
            grid_spacing: grid.spacing,
            dot_radius: grid.radius,
            background: scene.grid.background,
            dot: [r, g, b, a * grid.alpha],
            encode_srgb: if encode_srgb { 1.0 } else { 0.0 },
            padding: [0.0; 3],
        }
    }
}

/// One textured quad: a page view or a popup, in canvas space.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub(crate) struct QuadInstance {
    /// `x, y, width, height` in canvas units.
    pub(crate) rect: [f32; 4],
    /// Corner radius in canvas units (scaled by zoom in the shader).
    pub(crate) corner_radius: f32,
    padding: [f32; 3],
}

impl QuadInstance {
    /// A quad at `origin`/`size` with rounded corners.
    pub(crate) fn new(origin: Vec2, size: Vec2, corner_radius: f32) -> Self {
        Self {
            rect: [origin.x, origin.y, size.x, size.y],
            corner_radius,
            padding: [0.0; 3],
        }
    }

    /// Vertex attributes: `@location(0) rect`, `@location(1) corner_radius`.
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32];
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
    /// Outside stroke width in logical pixels.
    pub(crate) stroke_width: f32,
    padding: [f32; 2],
}

impl ShapeInstance {
    /// An instance with every field resolved.
    pub(crate) fn new(
        centre: Vec2,
        half_size: Vec2,
        corner_radius: f32,
        fill: [f32; 4],
        stroke: [f32; 4],
        stroke_width: f32,
    ) -> Self {
        Self {
            centre: centre.to_array(),
            half_size: half_size.to_array(),
            fill,
            stroke,
            corner_radius,
            stroke_width,
            padding: [0.0; 2],
        }
    }

    /// Vertex attributes: `@location(0)` centre and half size, `@location(1)`
    /// fill, `@location(2)` stroke, `@location(3)` radius and stroke width.
    pub(crate) const ATTRIBUTES: [wgpu::VertexAttribute; 4] =
        wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x2];
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
