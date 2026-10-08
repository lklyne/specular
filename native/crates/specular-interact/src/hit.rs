//! [`hit_test`]: what is under a screen point.

use glam::Vec2;
use specular_doc::{EdgeId, EdgeSide, Entity, EntityId, ItemId, Kind};

use crate::app::page_of;
use crate::geometry::ScreenRect;
use crate::{App, Handle, HandleOwner, PagePlacement, anchors, geometry, handles};

/// A drawing's box is only as thick as its ink (zero for a flat line), so it
/// is widened to at least this many logical pixels each way.
const DRAWING_MIN_HIT: f32 = 12.0;
/// How far in from a group's edge a press still counts as its border.
const GROUP_BORDER: f32 = 8.0;
/// The box of the group title above its top-left corner: one line of 11 px
/// text and the gap under it.
const GROUP_LABEL_HEIGHT: f32 = crate::edit::TITLE_LINE + crate::edit::TITLE_GAP;
/// Average advance of the title's glyphs, standing in for measured text.
const GROUP_LABEL_CHAR_WIDTH: f32 = 6.1;

/// What a screen point lands on.
///
/// Match on this without a wildcard arm, so a new target makes the compiler
/// list every place that must handle it.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    /// A group's title, above its top-left corner.
    GroupLabel {
        /// The group.
        group: EntityId,
    },
    /// A resize handle of the selection.
    Handle {
        /// What the handle resizes.
        owner: HandleOwner,
        /// Which handle.
        handle: Handle,
    },
    /// An edge anchor beside the selected or hovered entity.
    Anchor {
        /// The entity an edge would start from.
        entity: EntityId,
        /// The side the anchor is on.
        side: EdgeSide,
    },
    /// A page's content.
    PageContent {
        /// The page entity.
        page: EntityId,
        /// The point in the page's CSS pixels.
        local: Vec2,
    },
    /// The body of any other entity. For a group this is its interior, where
    /// no member is.
    EntityBody {
        /// The entity.
        entity: EntityId,
    },
    /// The band just inside a group's edge.
    GroupBorder {
        /// The group.
        group: EntityId,
    },
    /// An edge's line.
    Edge {
        /// The edge.
        edge: EdgeId,
    },
    /// Empty canvas.
    Empty,
}

/// What is under `screen`. Chrome comes first (group titles, then the
/// selection's resize handles, then edge anchors), then the bodies as
/// [`body_at`] orders them.
pub fn hit_test(app: &App, screen: Vec2) -> Hit {
    let camera = &app.session.camera;
    let document = &app.document;

    let label = document.entities().rev().find(|entity| {
        group_label_rect(entity, ScreenRect::of(camera, entity.rect))
            .is_some_and(|rect| rect.contains(screen))
    });
    if let Some(group) = label {
        return Hit::GroupLabel {
            group: group.id.clone(),
        };
    }

    if let Some((owner, bounds)) = app.handles()
        && let Some(handle) = handles::hit(ScreenRect::of(camera, bounds), screen)
    {
        return Hit::Handle { owner, handle };
    }

    if let Some((entity, side)) = anchors::at(app, screen) {
        return Hit::Anchor { entity, side };
    }

    body_at(app, screen)
}

/// The entity `hit` belongs to: the one whose body, title, handles or anchors
/// the point is on.
pub(crate) const fn entity_of(hit: &Hit) -> Option<&EntityId> {
    match hit {
        Hit::GroupLabel { group: entity }
        | Hit::GroupBorder { group: entity }
        | Hit::Handle {
            owner: HandleOwner::Entity(entity),
            ..
        }
        | Hit::Anchor { entity, .. }
        | Hit::PageContent { page: entity, .. }
        | Hit::EntityBody { entity } => Some(entity),
        Hit::Handle {
            owner: HandleOwner::Selection,
            ..
        }
        | Hit::Edge { .. }
        | Hit::Empty => None,
    }
}

/// The body under `screen`, ignoring chrome: entities and edges in stack
/// order from the front, then groups, the innermost first. Groups come last
/// whatever their place in the order, because they are containers and a
/// member inside one must be reachable.
pub(crate) fn body_at(app: &App, screen: Vec2) -> Hit {
    let camera = &app.session.camera;
    let document = &app.document;
    let mut groups = Vec::new();
    for item in document.order().iter().rev() {
        match item {
            ItemId::Edge(id) => {
                if app
                    .edge_curve(id)
                    .is_some_and(|curve| curve.hit(screen, camera.zoom))
                {
                    return Hit::Edge { edge: id.clone() };
                }
            }
            ItemId::Entity(id) => {
                let Some(entity) = document.entity(id) else {
                    continue;
                };
                let rect = ScreenRect::of(camera, entity.rect);
                let body = match &entity.kind {
                    Kind::Group(_) => {
                        groups.push((id, rect));
                        continue;
                    }
                    Kind::Drawing(_) => drawing_rect(rect),
                    Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Shape(_) => rect,
                };
                if body.contains(screen) {
                    return entity_hit(app, entity, screen);
                }
            }
        }
    }
    // The innermost group wins, then the one in front: a group's run puts it
    // behind nothing it holds, so stack order alone would hand the press to
    // the outer group and leave the inner one out of reach.
    let depth = |id: &EntityId| document.ancestors(id).count();
    groups
        .into_iter()
        .filter(|(_, rect)| rect.contains(screen))
        .reduce(|best, next| {
            if depth(next.0) > depth(best.0) {
                next
            } else {
                best
            }
        })
        .map_or(Hit::Empty, |(id, rect)| {
            let inside = rect.inflated(-GROUP_BORDER);
            let interior = screen.cmpgt(inside.min).all() && screen.cmplt(inside.max()).all();
            if interior {
                Hit::EntityBody { entity: id.clone() }
            } else {
                Hit::GroupBorder { group: id.clone() }
            }
        })
}

fn entity_hit(app: &App, entity: &Entity, screen: Vec2) -> Hit {
    match app.page_placement(&entity.id) {
        Some(placement) => {
            let world = app.session.camera.screen_to_world(screen).as_dvec2();
            Hit::PageContent {
                page: entity.id.clone(),
                local: placement.page_local(world).as_vec2(),
            }
        }
        None => Hit::EntityBody {
            entity: entity.id.clone(),
        },
    }
}

/// The frontmost page containing the canvas point `world`, whatever is on
/// top of it.
pub(crate) fn page_at(app: &App, world: glam::DVec2) -> Option<(EntityId, PagePlacement)> {
    let entity = app
        .document
        .entities()
        .rev()
        .find(|entity| page_of(entity).is_some() && geometry::contains(entity.rect, world))?;
    Some((entity.id.clone(), app.page_placement(&entity.id)?))
}

fn drawing_rect(rect: ScreenRect) -> ScreenRect {
    let size = rect.size.max(Vec2::splat(DRAWING_MIN_HIT));
    ScreenRect {
        min: rect.min - (size - rect.size) / 2.0,
        size,
    }
}

/// The title box of a labelled group whose rect is `rect` on screen.
fn group_label_rect(entity: &Entity, rect: ScreenRect) -> Option<ScreenRect> {
    let label = match &entity.kind {
        Kind::Group(_) => entity.label.as_deref().filter(|label| !label.is_empty())?,
        Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Drawing(_) | Kind::Shape(_) => {
            return None;
        }
    };
    let width = label.chars().count() as f32 * GROUP_LABEL_CHAR_WIDTH;
    Some(ScreenRect::new(
        rect.min.x,
        rect.min.y - GROUP_LABEL_HEIGHT,
        width,
        GROUP_LABEL_HEIGHT,
    ))
}
