//! What the caller asks [`Compositor::render`](crate::Compositor::render) to
//! draw, and what it reports back.

use std::time::Duration;

use glam::Vec2;
use specular_core::{Camera, CanvasRect, PageId};

/// One page to draw this frame, in paint order (back to front).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageDraw {
    /// Which page's latest frame to draw.
    pub page: PageId,
    /// Where, in canvas space.
    pub rect: CanvasRect,
}

/// Dot-grid background parameters (canvas-space spacing, like canvas-bg).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DotGrid {
    /// Distance between dots in canvas units (`GRID_SIZE`). The on-screen
    /// step doubles when zoomed out so dots never get denser than 8px.
    pub spacing: f32,
    /// Dot radius in screen (logical) pixels, snapped to device pixels.
    pub radius: f32,
    /// Background colour, linear RGBA.
    pub background: [f32; 4],
    /// Dot colour, linear RGBA.
    pub dot: [f32; 4],
}

impl Default for DotGrid {
    fn default() -> Self {
        Self {
            spacing: 20.0,
            radius: 0.7,
            background: [0.91, 0.91, 0.91, 1.0],
            dot: [0.45, 0.45, 0.45, 1.0],
        }
    }
}

/// Everything [`Compositor::render`](crate::Compositor::render) needs for one
/// window frame.
#[derive(Debug, Clone, Copy)]
pub struct SceneView<'a> {
    /// Canvas camera.
    pub camera: Camera,
    /// Viewport size in logical pixels.
    pub viewport: Vec2,
    /// Physical pixels per logical pixel (window scale factor).
    pub scale_factor: f32,
    /// Pages in paint order.
    pub pages: &'a [PageDraw],
    /// Background grid.
    pub grid: DotGrid,
}

/// Per-frame counters, comparable to the Electron lab's benchmark fields.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RenderStats {
    /// Pages with a texture that were drawn (after culling).
    pub pages_drawn: u32,
    /// Visible pages that had no frame yet (`framesWithoutTexture`).
    pub pages_without_texture: u32,
    /// Pages whose current texture came from a CPU upload (non-representative).
    pub cpu_textures: u32,
    /// Page frames drawn for the first time this render.
    pub new_frames_shown: u32,
    /// Longest wait, among frames shown for the first time this render,
    /// between the source receiving the paint and this frame's submit.
    pub max_paint_to_submit: Option<Duration>,
    /// CPU time spent encoding and submitting the frame.
    pub encode_time: Duration,
}
