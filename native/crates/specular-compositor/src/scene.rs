//! The background grid, and the counters every render reports.

use std::time::Duration;

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

/// Per-frame counters, comparable to the Electron lab's benchmark fields
/// and the page host's `PageHostStats`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RenderStats {
    /// Visible pages that had no frame yet (`framesWithoutTexture`).
    pub pages_without_texture: u32,
    /// Pages whose current texture came from a CPU upload (non-representative).
    pub cpu_textures: u32,
    /// Shapes drawn, after culling those outside the viewport.
    pub shapes_drawn: u32,
    /// Longest wait, among frames shown for the first time this render,
    /// between the source receiving the paint and this frame's submit.
    pub max_paint_to_submit: Option<Duration>,
    /// View-layer frames ingested since the previous render.
    pub frames_received: u32,
    /// Popup-layer frames ingested since the previous render.
    pub popup_frames: u32,
    /// Paints the source refused at the outstanding-texture cap since the
    /// previous render.
    pub frames_dropped_for_pool_pressure: u32,
    /// Shared textures held after this render, summed over pages.
    pub outstanding_textures: u32,
    /// Shared textures held by the page holding the most.
    pub max_outstanding_textures: u32,
}
