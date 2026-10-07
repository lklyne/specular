//! [`hit_test`]: what is under a screen point.

use glam::{DVec2, Vec2};
use specular_doc::EntityId;

use crate::app::page_of;
use crate::{App, Corner, PagePlacement, geometry, handles};

/// What a screen point lands on, topmost first.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    /// A resize handle of the selected entity.
    Handle {
        /// The entity the handle resizes.
        entity: EntityId,
        /// Which handle.
        corner: Corner,
    },
    /// A page's content.
    PageContent {
        /// The page entity.
        page: EntityId,
        /// The point in the page's CSS pixels.
        local: Vec2,
    },
    /// Empty canvas.
    Empty,
}

/// What is under `screen`. Handles sit above everything, then entities in
/// stack order from the front.
pub fn hit_test(app: &App, screen: Vec2) -> Hit {
    let camera = &app.session.camera;
    if let Some(target) = handles::resize_target(app)
        && let Some(corner) = handles::hit(target.rect, camera, screen)
    {
        return Hit::Handle {
            entity: target.entity.clone(),
            corner,
        };
    }
    let world = camera.screen_to_world(screen).as_dvec2();
    match page_at(app, world) {
        Some((page, placement)) => Hit::PageContent {
            page,
            local: placement.page_local(world).as_vec2(),
        },
        None => Hit::Empty,
    }
}

/// The frontmost page containing the canvas point `world`.
pub(crate) fn page_at(app: &App, world: DVec2) -> Option<(EntityId, PagePlacement)> {
    let entity = app
        .document
        .entities()
        .rev()
        .find(|entity| page_of(entity).is_some() && geometry::contains(entity.rect, world))?;
    Some((entity.id.clone(), app.page_placement(&entity.id)?))
}
