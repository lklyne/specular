//! The canvas camera: a uniform zoom plus a screen-space pan.
//!
//! The mapping is `screen = world * zoom + pan`, identical to the Electron
//! app's runtime camera, so a gesture profile replayed here moves the canvas
//! exactly as `/perf/pan-zoom/run` moves it there. Screen space is logical
//! (DPI-independent) pixels with the origin at the canvas viewport's top-left;
//! the compositor applies the window scale factor on top.

use glam::{Mat4, Vec2, Vec3};

use crate::geometry::CanvasRect;

/// Smallest zoom the canvas allows (`CANVAS_MIN_ZOOM` in `src/shared/zoom.ts`).
pub const MIN_ZOOM: f32 = 0.02;
/// Largest zoom the canvas allows (`CANVAS_MAX_ZOOM` in `src/shared/zoom.ts`).
pub const MAX_ZOOM: f32 = 3.0;
/// Zoom change per unit of wheel `deltaY` (`applyViewportInputDelta`).
pub const ZOOM_PER_WHEEL_DELTA: f32 = 0.002;

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

    /// Projects a canvas rect to its on-screen rect (still expressed as a
    /// [`CanvasRect`], in logical screen pixels).
    pub fn rect_to_screen(&self, rect: CanvasRect) -> CanvasRect {
        let origin = self.world_to_screen(rect.origin());
        let size = rect.size() * self.zoom;
        CanvasRect::new(origin.x, origin.y, size.x, size.y)
    }

    /// The world-space rect visible through a viewport of `viewport` logical
    /// pixels; used for culling and painting policy.
    pub fn visible_world_rect(&self, viewport: Vec2) -> CanvasRect {
        let origin = self.screen_to_world(Vec2::ZERO);
        let size = viewport / self.zoom;
        CanvasRect::new(origin.x, origin.y, size.x, size.y)
    }

    /// Pans by a screen-space delta in logical pixels.
    pub fn pan_by(&mut self, delta: Vec2) {
        self.pan += delta;
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
pub fn clamp_zoom(zoom: f32) -> f32 {
    zoom.clamp(MIN_ZOOM, MAX_ZOOM)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: Vec2, b: Vec2) {
        assert!(a.abs_diff_eq(b, 1e-3), "{a} != {b}");
    }

    #[test]
    fn screen_to_world_inverts_world_to_screen() {
        let camera = Camera::new(Vec2::new(120.0, -40.0), 0.37);
        let world = Vec2::new(812.5, -33.0);
        assert_close(camera.screen_to_world(camera.world_to_screen(world)), world);
    }

    #[test]
    fn zoom_about_keeps_anchor_world_point_fixed() {
        let mut camera = Camera::new(Vec2::new(30.0, 10.0), 1.0);
        let anchor = Vec2::new(400.0, 300.0);
        let before = camera.screen_to_world(anchor);
        camera.zoom_about(anchor, 2.5);
        assert_close(camera.screen_to_world(anchor), before);
    }

    #[test]
    fn negative_wheel_delta_zooms_in_by_electron_factor() {
        let mut camera = Camera::default();
        camera.apply_input_delta(ViewportInputDelta {
            zoom_delta_y: -100.0,
            ..ViewportInputDelta::default()
        });
        assert!((camera.zoom - 1.2).abs() < 1e-6);
    }

    #[test]
    fn apply_input_delta_clamps_zoom_to_minimum() {
        let mut camera = Camera::default();
        camera.apply_input_delta(ViewportInputDelta {
            zoom_delta_y: 10_000.0,
            ..ViewportInputDelta::default()
        });
        assert!((camera.zoom - MIN_ZOOM).abs() < f32::EPSILON);
    }

    #[test]
    fn view_projection_maps_viewport_corners_to_clip_corners() {
        let camera = Camera::new(Vec2::new(50.0, 25.0), 2.0);
        let viewport = Vec2::new(800.0, 600.0);
        let m = camera.view_projection(viewport);
        let world_bottom_right = camera.screen_to_world(viewport);
        let clip = m.project_point3(world_bottom_right.extend(0.0));
        assert_close(clip.truncate(), Vec2::new(1.0, -1.0));
    }

    #[test]
    fn visible_world_rect_spans_viewport_divided_by_zoom() {
        let camera = Camera::new(Vec2::ZERO, 0.5);
        let rect = camera.visible_world_rect(Vec2::new(800.0, 600.0));
        assert_close(rect.size(), Vec2::new(1600.0, 1200.0));
    }
}
