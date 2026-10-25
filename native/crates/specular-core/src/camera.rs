//! The canvas camera: a uniform zoom plus a screen-space pan.
//!
//! The mapping is `screen = world * zoom + pan`, identical to the Electron
//! app's runtime camera, so a gesture profile replayed here moves the canvas
//! exactly as `/perf/pan-zoom/run` moves it there. Screen space is logical
//! (DPI-independent) pixels with the origin at the canvas viewport's top-left;
//! the compositor applies the window scale factor on top.

use glam::{Mat4, Vec2, Vec3};

use crate::geometry::Rect;

/// Smallest zoom the canvas allows (`CANVAS_MIN_ZOOM` in `src/shared/zoom.ts`).
pub(crate) const MIN_ZOOM: f32 = 0.02;
/// Largest zoom the canvas allows (`CANVAS_MAX_ZOOM` in `src/shared/zoom.ts`).
pub(crate) const MAX_ZOOM: f32 = 3.0;
/// Zoom change per unit of wheel `deltaY` (`applyViewportInputDelta`).
pub(crate) const ZOOM_PER_WHEEL_DELTA: f32 = 0.002;

/// One tick of viewport input: what a wheel/trackpad event or one gesture
/// profile step applies to the camera.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ViewportInputDelta {
    /// Screen-space pan to add, in logical pixels.
    pub pan: Vec2,
    /// Wheel-style zoom delta; negative zooms in (`zoom -= delta * 0.002`).
    pub zoom_delta_y: f32,
    /// Screen-space point that stays fixed while zooming. `None` keeps the
    /// pan unchanged by the zoom, matching the Electron path with no mouse.
    pub anchor: Option<Vec2>,
}

/// Canvas camera state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Screen-space offset of the world origin, in logical pixels.
    pub pan: Vec2,
    /// Screen pixels per world unit, clamped to [`MIN_ZOOM`]..=[`MAX_ZOOM`].
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            zoom: 1.0,
        }
    }
}

impl Camera {
    /// Creates a camera, clamping `zoom` into the allowed range.
    pub fn new(pan: Vec2, zoom: f32) -> Self {
        Self {
            pan,
            zoom: clamp_zoom(zoom),
        }
    }

    /// Projects a world (canvas-space) point to screen space.
    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        world * self.zoom + self.pan
    }

    /// Unprojects a screen-space point to world (canvas) space.
    pub fn screen_to_world(&self, screen: Vec2) -> Vec2 {
        (screen - self.pan) / self.zoom
    }

    /// The world-space rect visible through a viewport of `viewport` logical
    /// pixels; used for culling and painting policy.
    pub fn visible_world_rect(&self, viewport: Vec2) -> Rect {
        let origin = self.screen_to_world(Vec2::ZERO);
        let size = viewport / self.zoom;
        Rect::new(origin.x, origin.y, size.x, size.y)
    }

    /// Whether any part of `rect` (canvas space) shows through a viewport of
    /// `viewport` logical pixels. Pages that fail this are culled: not drawn,
    /// and candidates for pausing paint.
    pub fn is_visible(&self, rect: Rect, viewport: Vec2) -> bool {
        self.visible_world_rect(viewport).intersects(rect)
    }

    /// Sets the zoom (clamped), keeping the world point under `anchor`
    /// (screen space) fixed on screen.
    pub fn zoom_about(&mut self, anchor: Vec2, zoom: f32) {
        let world = self.screen_to_world(anchor);
        self.zoom = clamp_zoom(zoom);
        self.pan = anchor - world * self.zoom;
    }

    /// Applies one tick of viewport input, mirroring the Electron runtime's
    /// `applyViewportInputDelta`: zoom first (about the anchor when given),
    /// then add the pan.
    pub fn apply_input_delta(&mut self, delta: ViewportInputDelta) {
        if delta.zoom_delta_y != 0.0 {
            let next = self.zoom - delta.zoom_delta_y * ZOOM_PER_WHEEL_DELTA;
            match delta.anchor {
                Some(anchor) => self.zoom_about(anchor, next),
                None => self.zoom = clamp_zoom(next),
            }
        }
        self.pan += delta.pan;
    }

    /// World -> clip-space matrix for a viewport of `viewport` logical pixels
    /// (wgpu clip space: x right, y up, origin at the viewport centre).
    pub fn view_projection(&self, viewport: Vec2) -> Mat4 {
        let safe = viewport.max(Vec2::ONE);
        let to_clip = Mat4::from_translation(Vec3::new(-1.0, 1.0, 0.0))
            * Mat4::from_scale(Vec3::new(2.0 / safe.x, -2.0 / safe.y, 1.0));
        let world_to_screen = Mat4::from_translation(self.pan.extend(0.0))
            * Mat4::from_scale(Vec3::new(self.zoom, self.zoom, 1.0));
        to_clip * world_to_screen
    }
}

/// Clamps a zoom value into [`MIN_ZOOM`]..=[`MAX_ZOOM`].
pub(crate) fn clamp_zoom(zoom: f32) -> f32 {
    zoom.clamp(MIN_ZOOM, MAX_ZOOM)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Zoom = fn(&mut Camera, Vec2);

    #[test]
    fn zoom_keeps_the_point_under_the_anchor_fixed() {
        let rows: [(&str, Camera, Vec2, Zoom); 3] = [
            (
                "zoom_about in",
                Camera::new(Vec2::new(30.0, 10.0), 1.0),
                Vec2::new(400.0, 300.0),
                |camera, anchor| camera.zoom_about(anchor, 2.5),
            ),
            (
                "zoom_about past the limit",
                Camera::new(Vec2::new(-10.0, 5.0), 2.0),
                Vec2::new(123.0, 456.0),
                |camera, anchor| camera.zoom_about(anchor, 50.0),
            ),
            (
                "wheel input",
                Camera::new(Vec2::new(200.0, -80.0), 0.75),
                Vec2::new(512.0, 384.0),
                |camera, anchor| {
                    camera.apply_input_delta(ViewportInputDelta {
                        zoom_delta_y: -37.0,
                        anchor: Some(anchor),
                        ..ViewportInputDelta::default()
                    });
                },
            ),
        ];
        for (name, mut camera, anchor, zoom) in rows {
            let before = camera.screen_to_world(anchor);
            zoom(&mut camera, anchor);
            assert!(
                camera.screen_to_world(anchor).abs_diff_eq(before, 1e-3),
                "{name}"
            );
        }
    }

    #[test]
    fn wheel_delta_zooms_by_the_electron_factor_within_the_limits() {
        let rows = [
            ("zoom in", -100.0, 1.2),
            ("past the minimum", 10_000.0, MIN_ZOOM),
            ("past the maximum", -10_000.0, MAX_ZOOM),
        ];
        for (name, zoom_delta_y, expected) in rows {
            let mut camera = Camera::default();
            camera.apply_input_delta(ViewportInputDelta {
                zoom_delta_y,
                ..ViewportInputDelta::default()
            });
            assert!(
                (camera.zoom - expected).abs() < 1e-6,
                "{name}: {}",
                camera.zoom
            );
        }
        assert!((Camera::new(Vec2::ZERO, 0.0).zoom - MIN_ZOOM).abs() < f32::EPSILON);
    }
}
