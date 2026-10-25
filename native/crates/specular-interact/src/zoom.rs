//! The View menu's camera commands: zoom in and out in steps, back to 100%,
//! and zoom to fit everything on the canvas.

use glam::DVec2;
use specular_core::Camera;
use specular_doc::Rect;

use crate::viewport::{area, centre};
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
    to(app, 100);
}

/// Zooms to `percent`, about the middle of the viewport.
pub(crate) fn to(app: &mut App, percent: u16) {
    let about = centre(app);
    (app.session.camera).zoom_about(about, f32::from(percent) / 100.0);
}

fn scale(app: &mut App, by: f32) {
    let about = centre(app);
    let camera = &mut app.session.camera;
    let zoom = camera.zoom * by;
    camera.zoom_about(about, zoom);
}

/// Shows every entity, centred, as large as fits. An empty canvas goes back
/// to its origin at 100%. With the chrome over the top of the viewport and
/// the sidebar over its left edge, the fit is of what is left free, as the
/// Electron app's canvas view starts below its toolbar and right of its
/// sidebar.
pub(crate) fn to_fit(app: &mut App) {
    let bounds = (app.document.entities())
        .map(|entity| entity.rect)
        .reduce(Rect::union);
    let free = area(app);
    app.session.camera = match bounds {
        Some(bounds) => {
            let mut camera = fitting(bounds, free.size);
            camera.pan += free.min.as_vec2();
            camera
        }
        None => Camera::default(),
    };
}

/// Pans, without zooming, by the least that brings `bounds` into the
/// viewport with [`REVEAL_PADDING`] around it. Something larger than the
/// viewport shows its top-left corner. Already in view, nothing moves.
pub(crate) fn reveal(app: &mut App, bounds: Rect) {
    let free = area(app);
    if free.is_empty() {
        return;
    }
    let camera = &mut app.session.camera;
    let zoom = f64::from(camera.zoom);
    let pad = DVec2::splat(f64::from(REVEAL_PADDING)).min(free.size / 4.0);
    // Measured from the free part's own corner.
    let low = geometry::origin(bounds) * zoom + camera.pan.as_dvec2() - free.min;
    let high = low + geometry::size(bounds) * zoom;
    // Past the far side, come back by the overshoot; then never leave the
    // near side cut off.
    let back = (high - (free.size - pad)).max(DVec2::ZERO);
    let shift = (pad - (low - back)).max(DVec2::ZERO) - back;
    camera.pan += shift.as_vec2();
}

/// What a fit has to fill in a viewport of `viewport` logical pixels: all
/// of it but [`FIT_PADDING`] on each side.
pub(crate) fn fit_room(viewport: DVec2) -> DVec2 {
    (viewport - DVec2::splat(f64::from(FIT_PADDING) * 2.0)).max(DVec2::ONE)
}

/// The camera that centres `bounds` in a viewport of `viewport` logical
/// pixels with [`FIT_PADDING`] around it.
pub fn fitting(bounds: Rect, viewport: DVec2) -> Camera {
    let room = fit_room(viewport);
    let size = geometry::size(bounds).max(DVec2::ONE);
    let zoom = (room / size).min_element().min(f64::from(FIT_MAX_ZOOM)) as f32;
    let camera = Camera::new(glam::Vec2::ZERO, zoom);
    let centre = geometry::origin(bounds) + geometry::size(bounds) / 2.0;
    Camera {
        pan: (viewport / 2.0 - centre * f64::from(camera.zoom)).as_vec2(),
        ..camera
    }
}
