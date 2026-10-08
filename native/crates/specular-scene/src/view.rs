//! [`view`]: the pure function from an [`App`] to the [`Scene`] one frame
//! draws.
//!
//! Entities and edges are drawn in the document's stack order, one module per
//! kind. The session layer goes over them: comment regions, badges and the
//! marker of the comment being written, the hover border, selection
//! outlines, resize handles, the marquee and the comment tool's preview,
//! and last the composer that comment is typed in. An entity whose text is being edited draws the
//! working text with its selection and caret. Nothing here touches a GPU,
//! and the only text measured is the one being edited, through the app's
//! own measure, so a scene can be built and compared in a test. What costs
//! most to build (a Document's rows, a stroke's outline, the edited
//! Document's layout) is kept in the caller's [`ViewCache`].
//!
//! Content is in canvas space. Chrome that keeps its pixel size at any zoom
//! is projected with the camera and emitted in screen space.

mod annotations;
mod comment_draft;
mod document;
mod drawing;
mod edge;
mod edge_chrome;
mod editing;
mod file;
mod frame;
mod freehand;
mod group;
mod guides;
mod image;
mod layout_handles;
mod page;
pub(crate) mod palette;
mod session;
mod shape;
mod shape_path;
mod text;

use glam::Vec2;
use specular_doc::{Entity, ItemId, Kind};
use specular_interact::App;

use self::frame::Frame;
use crate::{Scene, ViewCache};

/// Everything `app` shows in a `viewport` of logical pixels, back to front.
///
/// Entities wholly outside the viewport are left out, so the cost of a frame
/// follows what is on screen and not the size of the document. `cache` is
/// the caller's, kept from one frame to the next; the scene is the same
/// whatever it holds.
pub fn view(app: &App, viewport: Vec2, cache: &ViewCache) -> Scene {
    build(&Frame::new(app, viewport, true, cache))
}

/// [`view`] with the chrome left out: no page borders or titles and no
/// session layer. Entities and edges are still drawn. A benchmark uses it to
/// time the content alone.
pub fn view_without_chrome(app: &App, viewport: Vec2, cache: &ViewCache) -> Scene {
    build(&Frame::new(app, viewport, false, cache))
}

fn build(frame: &Frame<'_>) -> Scene {
    frame.cache.begin();
    let mut scene = Scene::new();
    let document = frame.app.document();
    // Tints go behind everything; a group's border and title wait for its
    // own slot, in front of its members.
    for group in group::backgrounds(document) {
        if frame.sees(group.rect) {
            group::draw_background(frame, group, &mut scene);
        }
    }
    for item in document.order() {
        match item {
            ItemId::Entity(id) => {
                if let Some(entity) = document.entity(id) {
                    draw_in_place(frame, entity, &mut scene);
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
        comment_draft::composer(frame, &mut scene);
    }
    scene
}

/// `entity` where its page's scroll has put it, drawn only through the page
/// while it is shifted. Nothing is drawn for one that is off screen or has
/// scrolled out of its page.
fn draw_in_place(frame: &Frame<'_>, entity: &Entity, scene: &mut Scene) {
    let Some(seen) = specular_interact::seen(frame.app, entity) else {
        return;
    };
    if !frame.sees(seen.entity.rect) {
        return;
    }
    let first = scene.items.len();
    draw_entity(frame, &seen.entity, scene);
    if let Some(page) = seen.clip {
        clip_from(frame, page, first, scene);
    }
}

/// Clips the items from `first` on to the canvas rect `page`, together with
/// whatever clip they have.
fn clip_from(frame: &Frame<'_>, page: specular_doc::Rect, first: usize, scene: &mut Scene) {
    let canvas = frame::canvas_rect(page);
    let screen = frame.screen_rect(page);
    for item in &mut scene.items[first..] {
        let through = match item.space {
            crate::Space::Canvas => canvas,
            crate::Space::Screen => screen,
        };
        item.clip = Some(match item.clip {
            Some(own) => own
                .intersection(through)
                .unwrap_or(crate::Rect::new(0.0, 0.0, 0.0, 0.0)),
            None => through,
        });
    }
}

fn draw_entity(frame: &Frame<'_>, entity: &Entity, scene: &mut Scene) {
    match &entity.kind {
        Kind::Page(page) => page::draw(frame, entity, page, scene),
        Kind::Text(text) => text::draw(frame, entity, text, scene),
        Kind::File(file) => file::draw(frame, entity, file, scene),
        Kind::Group(group) => group::draw(frame, entity, group, scene),
        Kind::Drawing(drawing) => drawing::draw(frame, entity, drawing, scene),
        Kind::Shape(shape) => shape::draw(frame, entity, shape, scene),
    }
}
