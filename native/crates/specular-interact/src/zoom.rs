//! The View menu's camera commands: zoom in and out in steps, back to 100%,
//! and zoom to fit everything on the canvas.

use glam::DVec2;
use specular_core::Camera;
use specular_doc::Rect;

use crate::panel::builtin::TOOLBAR_HEIGHT;
use crate::{App, geometry};

/// How much one zoom in or out changes the zoom.
const ZOOM_STEP: f32 = 1.25;
/// The room left around the canvas's contents by zoom to fit, in logical
/// pixels on each side.
const FIT_PADDING: f32 = 64.0;
/// The room [`reveal`] leaves between what it shows and the viewport's edge.
const REVEAL_PADDING: f32 = 48.0;
/// Zoom to fit never magnifies: a single small note stays its real size.
const FIT_MAX_ZOOM: f32 = 1.0;

/// Zooms one step in, about the middle of the viewport.
pub(crate) fn zoom_in(app: &mut App) {
    scale(app, ZOOM_STEP);
}

/// Zooms one step out, about the middle of the viewport.
pub(crate) fn zoom_out(app: &mut App) {
    scale(app, ZOOM_STEP.recip());
}

/// Zooms to 100%, about the middle of the viewport.
pub(crate) fn reset(app: &mut App) {
    let session = &mut app.session;
    session.camera.zoom_about(session.viewport / 2.0, 1.0);
}

fn scale(app: &mut App, by: f32) {
    let session = &mut app.session;
    let zoom = session.camera.zoom * by;
    session.camera.zoom_about(session.viewport / 2.0, zoom);
}

/// Shows every entity, centred, as large as fits. An empty canvas goes back
/// to its origin at 100%. With the built-in toolbar over the top of the
/// viewport, the fit is of what is left under it, as the Electron app's
/// canvas view starts below its toolbar.
pub(crate) fn to_fit(app: &mut App) {
    let bounds = (app.document.entities())
        .map(|entity| entity.rect)
        .reduce(geometry::union);
    let top = if app.session.panel.built_in {
        f64::from(TOOLBAR_HEIGHT)
    } else {
        0.0
    };
    app.session.camera = match bounds {
        Some(bounds) => {
            let below = app.session.viewport.as_dvec2() - DVec2::new(0.0, top);
            let mut camera = fitting(bounds, below);
            camera.pan.y += top as f32;
            camera
        }
        None => Camera::default(),
    };
}

/// Pans, without zooming, by the least that brings `bounds` into the
/// viewport with [`REVEAL_PADDING`] around it. Something larger than the
/// viewport shows its top-left corner. Already in view, nothing moves.
pub(crate) fn reveal(app: &mut App, bounds: Rect) {
    let camera = &mut app.session.camera;
    let zoom = f64::from(camera.zoom);
    let viewport = app.session.viewport.as_dvec2();
    if viewport.min_element() <= 0.0 {
        return;
    }
    let pad = DVec2::splat(f64::from(REVEAL_PADDING)).min(viewport / 4.0);
    let low = geometry::origin(bounds) * zoom + camera.pan.as_dvec2();
    let high = low + geometry::size(bounds) * zoom;
    // Past the far side, come back by the overshoot; then never leave the
    // near side cut off.
    let back = (high - (viewport - pad)).max(DVec2::ZERO);
    let shift = (pad - (low - back)).max(DVec2::ZERO) - back;
    camera.pan += shift.as_vec2();
}

/// The camera that centres `bounds` in a viewport of `viewport` logical
/// pixels with [`FIT_PADDING`] around it.
pub fn fitting(bounds: Rect, viewport: DVec2) -> Camera {
    let room = (viewport - DVec2::splat(f64::from(FIT_PADDING) * 2.0)).max(DVec2::ONE);
    let size = geometry::size(bounds).max(DVec2::ONE);
    let zoom = (room / size).min_element().min(f64::from(FIT_MAX_ZOOM)) as f32;
    let camera = Camera::new(glam::Vec2::ZERO, zoom);
    let centre = geometry::origin(bounds) + geometry::size(bounds) / 2.0;
    Camera {
        pan: (viewport / 2.0 - centre * f64::from(camera.zoom)).as_vec2(),
        ..camera
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contents_wider_than_the_viewport_are_scaled_down_and_centred() {
        let camera = fitting(
            Rect::new(100.0, 100.0, 2000.0, 400.0),
            DVec2::new(1128.0, 800.0),
        );
        assert!((camera.zoom - 0.5).abs() < 1e-6);
        // The contents' centre (1100, 300) lands on the viewport's (564, 400).
        assert_eq!(camera.pan, glam::Vec2::new(14.0, 250.0));

        {
            let camera = fitting(Rect::new(0.0, 0.0, 200.0, 100.0), DVec2::new(1000.0, 800.0));
            assert!((camera.zoom - 1.0).abs() < f32::EPSILON);
            assert_eq!(camera.pan, glam::Vec2::new(400.0, 350.0));
        }

        {
            let camera = fitting(Rect::new(0.0, 0.0, 1.0e7, 1.0e7), DVec2::new(1000.0, 800.0));
            let smallest = Camera::new(glam::Vec2::ZERO, 0.0).zoom;
            assert!((camera.zoom - smallest).abs() < f32::EPSILON);
        }
    }

    fn revealed(bounds: Rect) -> glam::Vec2 {
        let mut app = App::default();
        app.session.viewport = glam::Vec2::new(1000.0, 800.0);
        reveal(&mut app, bounds);
        app.session.camera.pan
    }

    #[test]
    fn revealing_pans_by_the_least_that_shows_it_with_room_around() {
        // 300 past the right edge, in view vertically.
        assert_eq!(
            revealed(Rect::new(1100.0, 100.0, 200.0, 200.0)),
            glam::Vec2::new(-348.0, 0.0)
        );
        // Above and to the left.
        assert_eq!(
            revealed(Rect::new(-500.0, -300.0, 200.0, 200.0)),
            glam::Vec2::new(548.0, 348.0)
        );

        {
            assert_eq!(
                revealed(Rect::new(100.0, 100.0, 200.0, 200.0)),
                glam::Vec2::ZERO
            );
        }

        {
            assert_eq!(
                revealed(Rect::new(2000.0, 100.0, 3000.0, 200.0)),
                glam::Vec2::new(-1952.0, 0.0)
            );
        }
    }
}
