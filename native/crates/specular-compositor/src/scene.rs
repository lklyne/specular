//! The background grid, and the counters every render reports.

use std::time::Duration;

use crate::scene_pass::linear;

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
    /// Opacity floor for dense grids.
    pub min_alpha: f32,
}

impl DotGrid {
    /// The grid of a theme: its canvas and its dots.
    #[must_use]
    pub fn themed(colors: &specular_scene::Colors) -> Self {
        Self {
            background: linear(colors.canvas, 1.0),
            dot: linear(colors.dot, 1.0),
            min_alpha: colors.dot_floor,
            ..Self::default()
        }
    }
}

impl Default for DotGrid {
    fn default() -> Self {
        Self {
            spacing: 20.0,
            radius: 0.7,
            // Electron's light theme: stone-200 at 60% over the window's
            // stone-100 is #edebea, and the dots are #a8a29e.
            background: [0.847, 0.831, 0.823, 1.0],
            dot: [0.392, 0.361, 0.342, 1.0],
            min_alpha: 0.52,
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
