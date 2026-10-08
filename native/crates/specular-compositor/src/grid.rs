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
/// Opacity floor for dense grids (light theme value).
const MIN_DOT_ALPHA: f32 = 0.52;
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
        alpha: (spacing / FULL_OPACITY_SPACING_PX).clamp(MIN_DOT_ALPHA, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics_at_zoom(zoom: f32) -> GridMetrics {
        grid_metrics(&Camera::new(Vec2::ZERO, zoom), &DotGrid::default(), 1.0)
    }

    #[test]
    fn spacing_doubles_until_at_least_minimum_when_zoomed_out() {
        // 20 * 0.1 = 2px -> x4 = 8px.
        assert!((metrics_at_zoom(0.1).spacing - 8.0).abs() < 1e-4);
    }

    #[test]
    fn step_multiplier_caps_at_sixty_four() {
        let grid = DotGrid {
            spacing: 1.0,
            ..DotGrid::default()
        };
        // 1 * 0.02 * 64 = 1.28px: still under the minimum, but capped.
        let metrics = grid_metrics(&Camera::new(Vec2::ZERO, 0.02), &grid, 1.0);
        assert!((metrics.spacing - 1.28).abs() < 1e-4);
    }

    #[test]
    fn radius_snaps_to_device_pixels_on_retina() {
        let metrics = grid_metrics(&Camera::default(), &DotGrid::default(), 2.0);
        // round(0.7 * 2) / 2 = 0.5, floored to 0.6.
        assert!((metrics.radius - MIN_DOT_RADIUS).abs() < f32::EPSILON);
    }

    #[test]
    fn non_positive_spacing_disables_dots() {
        let grid = DotGrid {
            spacing: 0.0,
            ..DotGrid::default()
        };
        let metrics = grid_metrics(&Camera::default(), &grid, 1.0);
        assert!(metrics.spacing.abs() < f32::EPSILON);
    }
}
