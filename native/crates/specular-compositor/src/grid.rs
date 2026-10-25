//! Dot-grid metrics, matching canvas-bg's `canvasGridStyle.ts`.
//!
//! The grid is drawn as one procedural full-screen fill (see
//! docs/perf-zoom-pan-log.md, Exp G: per-dot drawing was 44x slower than one
//! tiled fill), so all per-zoom decisions happen here once per frame.

use glam::Vec2;
use specular_core::Camera;

use crate::scene::DotGrid;

/// Below this on-screen spacing the grid doubles its step.
const MIN_GRID_SPACING_PX: f32 = 8.0;
/// On-screen spacing at which dots reach full opacity.
const FULL_OPACITY_SPACING_PX: f32 = 18.0;
/// Largest step doubling, so extreme zoom-out still shows a sparse grid.
const MAX_GRID_STEP_MULTIPLIER: f32 = 64.0;
/// Smallest dot radius in logical pixels.
const MIN_DOT_RADIUS: f32 = 0.6;

/// Per-frame grid parameters in logical screen pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct GridMetrics {
    /// Screen position of one dot (the world origin).
    pub(crate) origin: Vec2,
    /// Distance between dots; `0` disables the dots.
    pub(crate) spacing: f32,
    /// Dot radius, snapped so it covers whole device pixels.
    pub(crate) radius: f32,
    /// Multiplier for the dot colour's alpha.
    pub(crate) alpha: f32,
}

/// Grid parameters for `camera` on a display with `scale_factor`.
pub(crate) fn grid_metrics(camera: &Camera, grid: &DotGrid, scale_factor: f32) -> GridMetrics {
    let device_ratio = scale_factor.max(1.0);
    let radius = ((grid.radius * device_ratio).round() / device_ratio).max(MIN_DOT_RADIUS);
    if grid.spacing <= 0.0 || !grid.spacing.is_finite() {
        return GridMetrics {
            origin: camera.pan,
            spacing: 0.0,
            radius,
            alpha: 0.0,
        };
    }
    let mut multiplier = 1.0;
    while grid.spacing * camera.zoom * multiplier < MIN_GRID_SPACING_PX
        && multiplier < MAX_GRID_STEP_MULTIPLIER
    {
        multiplier *= 2.0;
    }
    let spacing = grid.spacing * camera.zoom * multiplier;
    GridMetrics {
        origin: camera.pan,
        spacing,
        radius,
        alpha: (spacing / FULL_OPACITY_SPACING_PX).clamp(grid.min_alpha, 1.0),
    }
}
