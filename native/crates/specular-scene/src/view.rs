//! [`view`]: the pure function from an [`App`] to the [`Scene`] one frame
//! draws.
//!
//! Entities and edges are drawn in the document's stack order, one module per
//! kind. The session layer goes over them: comment regions and badges, the
//! hover border, selection outlines, resize handles, the marquee and the
//! comment tool's preview. Nothing here measures text or touches a GPU, so a
//! scene can be built and compared in a test.
//!
//! Content is in canvas space. Chrome that keeps its pixel size at any zoom
//! is projected with the camera and emitted in screen space.

mod annotations;
mod drawing;
mod edge;
mod file;
mod frame;
mod freehand;
mod group;
mod image;
mod page;
mod palette;
mod session;
mod shape;
mod shape_path;
mod text;

use glam::Vec2;
use specular_doc::{Entity, ItemId, Kind};
use specular_interact::App;

use self::frame::Frame;
use crate::Scene;

/// Everything `app` shows in a `viewport` of logical pixels, back to front.
///
/// Entities wholly outside the viewport are left out, so the cost of a frame
/// follows what is on screen and not the size of the document.
pub fn view(app: &App, viewport: Vec2) -> Scene {
    build(&Frame::new(app, viewport, true))
}

/// [`view`] with the chrome left out: no page borders or titles and no
/// session layer. Entities and edges are still drawn. A benchmark uses it to
/// time the content alone.
pub fn view_without_chrome(app: &App, viewport: Vec2) -> Scene {
    build(&Frame::new(app, viewport, false))
}

fn build(frame: &Frame<'_>) -> Scene {
    let mut scene = Scene::new();
    let document = frame.app.document();
    for item in document.order() {
        match item {
            ItemId::Entity(id) => {
                if let Some(entity) = document.entity(id)
                    && frame.sees(entity.rect)
                {
                    draw_entity(frame, entity, &mut scene);
                }
            }
            ItemId::Edge(id) => {
                if let Some(edge) = document.edge(id) {
                    edge::draw(frame, edge, &mut scene);
                }
            }
        }
    }
    if frame.chrome {
        annotations::draw(frame, &mut scene);
        session::draw(frame, &mut scene);
    }
    scene
}

fn draw_entity(frame: &Frame<'_>, entity: &Entity, scene: &mut Scene) {
    match &entity.kind {
        Kind::Page(page) => page::draw(frame, entity, page, scene),
        Kind::Text(text) => text::draw(entity, text, scene),
        Kind::File(file) => file::draw(frame, entity, file, scene),
        Kind::Group(group) => group::draw(frame, entity, group, scene),
        Kind::Drawing(drawing) => drawing::draw(frame, entity, drawing, scene),
        Kind::Shape(shape) => shape::draw(entity, shape, scene),
    }
}
