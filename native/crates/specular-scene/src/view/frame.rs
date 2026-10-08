//! [`Frame`]: what every drawing function needs to know about the frame
//! being built, and the projection from canvas space to the screen.

use glam::{DVec2, Vec2};
use specular_core::Camera;
use specular_interact::App;

use crate::{Point, Rect};

/// How far outside the viewport, in logical pixels, an entity may sit and
/// still be drawn. It covers the chrome around a rect: a group's title, a
/// page's title, outlines and handles.
const CULL_MARGIN: f32 = 64.0;

/// One frame being built.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Frame<'a> {
    /// The app the frame shows.
    pub(crate) app: &'a App,
    /// Whether chrome is drawn.
    pub(crate) chrome: bool,
    camera: Camera,
    /// The part of the canvas that may be on screen, margin included.
    visible: Rect,
    /// The viewport in screen space, margin included.
    screen: Rect,
}

impl<'a> Frame<'a> {
    pub(crate) fn new(app: &'a App, viewport: Vec2, chrome: bool) -> Self {
        let camera = app.session().camera;
        let world = camera.visible_world_rect(viewport);
        let visible = Rect::new(world.x, world.y, world.width, world.height)
            .outset(CULL_MARGIN / camera.zoom.max(f32::EPSILON));
        Self {
            app,
            chrome,
            camera,
            visible,
            screen: Rect::new(0.0, 0.0, viewport.x, viewport.y).outset(CULL_MARGIN),
        }
    }

    /// The same frame with no chrome drawn.
    pub(crate) fn without_chrome(self) -> Self {
        Self {
            chrome: false,
            ..self
        }
    }

    /// Logical pixels per canvas unit.
    pub(crate) fn zoom(&self) -> f32 {
        self.camera.zoom
    }

    /// Whether anything at `rect` could be on screen. A rect with no area (a
    /// flat stroke, an empty group) still counts where it sits.
    pub(crate) fn sees(&self, rect: specular_doc::Rect) -> bool {
        canvas_rect(rect).outset(0.5).intersects(self.visible)
    }

    /// Whether a screen-space rect could be on screen.
    pub(crate) fn sees_screen(&self, rect: Rect) -> bool {
        rect.outset(0.5).intersects(self.screen)
    }

    /// A canvas point on screen.
    pub(crate) fn screen_point(&self, point: DVec2) -> Point {
        vec_point(self.camera.world_to_screen(point.as_vec2()))
    }

    /// A scene rect in canvas space, on screen.
    pub(crate) fn project(&self, rect: Rect) -> Rect {
        let origin = vec_point(self.camera.world_to_screen(Vec2::new(rect.x, rect.y)));
        Rect::new(
            origin.x,
            origin.y,
            rect.width * self.camera.zoom,
            rect.height * self.camera.zoom,
        )
    }

    /// A canvas rect on screen.
    pub(crate) fn screen_rect(&self, rect: specular_doc::Rect) -> Rect {
        let origin = self.screen_point(DVec2::new(rect.x, rect.y));
        Rect::new(
            origin.x,
            origin.y,
            rect.width as f32 * self.camera.zoom,
            rect.height as f32 * self.camera.zoom,
        )
    }
}

/// A document rect as a scene rect in canvas space.
pub(crate) fn canvas_rect(rect: specular_doc::Rect) -> Rect {
    Rect::new(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
    )
}

pub(crate) fn vec_point(vec: Vec2) -> Point {
    Point::new(vec.x, vec.y)
}
