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
/// to its origin at 100%. With the built-in toolbar over the top of the
/// viewport and the sidebar over its left edge, the fit is of what is left
/// free, as the Electron app's canvas view starts below its toolbar and
/// right of its sidebar.
pub(crate) fn to_fit(app: &mut App) {
    let bounds = (app.document.entities())
        .map(|entity| entity.rect)
        .reduce(geometry::union);
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

/// Frames the selected items, as large as fits in the part of the viewport
/// the toolbar and the sidebar leave free. Does nothing with nothing
/// selected.
pub(crate) fn focus_selection(app: &mut App) {
    let scope = app.selection_scope();
    let Some(bounds) = scope.shown_bounds.or(scope.bounds) else {
        return;
    };
    let free = area(app);
    let mut camera = fitting(bounds, free.size);
    camera.pan += free.min.as_vec2();
    app.session.camera = camera;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contents_are_scaled_down_to_fit_and_centred() {
        let smallest = Camera::new(glam::Vec2::ZERO, 0.0).zoom;
        // (contents, viewport, zoom, pan). The first centres (1100, 300) on
        // the viewport's (564, 400); the last is clamped to the smallest zoom.
        let rows = [
            (
                Rect::new(100.0, 100.0, 2000.0, 400.0),
                (1128.0, 800.0),
                0.5,
                Some(glam::Vec2::new(14.0, 250.0)),
            ),
            (
                Rect::new(0.0, 0.0, 200.0, 100.0),
                (1000.0, 800.0),
                1.0,
                Some(glam::Vec2::new(400.0, 350.0)),
            ),
            (
                Rect::new(0.0, 0.0, 1.0e7, 1.0e7),
                (1000.0, 800.0),
                smallest,
                None,
            ),
        ];
        for (contents, (width, height), zoom, pan) in rows {
            let camera = fitting(contents, DVec2::new(width, height));
            assert!((camera.zoom - zoom).abs() < 1e-6, "{contents:?}");
            if let Some(pan) = pan {
                assert_eq!(camera.pan, pan, "{contents:?}");
            }
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
        let rows = [
            // 300 past the right edge, in view vertically.
            (Rect::new(1100.0, 100.0, 200.0, 200.0), (-348.0, 0.0)),
            // Above and to the left.
            (Rect::new(-500.0, -300.0, 200.0, 200.0), (548.0, 348.0)),
            // Already in view.
            (Rect::new(100.0, 100.0, 200.0, 200.0), (0.0, 0.0)),
            // Wider than the viewport: its left edge wins.
            (Rect::new(2000.0, 100.0, 3000.0, 200.0), (-1952.0, 0.0)),
        ];
        for (bounds, (x, y)) in rows {
            assert_eq!(revealed(bounds), glam::Vec2::new(x, y), "{bounds:?}");
        }
    }
}
