//! [`hit_test`]: what is under a screen point.

use glam::Vec2;
use specular_doc::{AnnotationId, Drawing, EdgeId, EdgeSide, Entity, EntityId, ItemId, Kind};

use crate::app::page_of;
use crate::edge_path::distance_to_segment;
use crate::geometry::ScreenRect;
use crate::{App, Handle, HandleOwner, PagePlacement, anchors, comment, geometry, handles};

/// A drawing's box is only as thick as its ink (zero for a flat line), so it
/// is widened to at least this many logical pixels each way. A stroke is
/// hit within half this of its line, however thin it is.
const DRAWING_MIN_HIT: f32 = 12.0;
/// A stroke is drawn this much wider than its nominal width.
const INK_SCALE: f32 = 1.6;
/// How far in from a group's edge a press still counts as its border.
const GROUP_BORDER: f32 = 8.0;
/// The box of the group title above its top-left corner: one line of 11 px
/// text and the gap under it.
const GROUP_LABEL_HEIGHT: f32 = crate::edit::TITLE_LINE + crate::edit::TITLE_GAP;
/// Average advance of the title's glyphs, standing in for measured text.
const GROUP_LABEL_CHAR_WIDTH: f32 = 6.1;
/// Below this zoom the titles above groups and pages stop holding their
/// pixel size and shrink with the canvas, so a title is never much larger
/// than the thing it names.
const TITLE_FULL_ZOOM: f32 = 0.5;

/// The size of a group's or a page's title at `zoom`, as a fraction of its
/// full size. Drawing and hit-testing both use it.
pub fn title_scale(zoom: f32) -> f32 {
    (zoom / TITLE_FULL_ZOOM).clamp(0.0, 1.0)
}

/// What a screen point lands on.
///
/// Match on this without a wildcard arm, so a new target makes the compiler
/// list every place that must handle it.
#[derive(Debug, Clone, PartialEq)]
pub enum Hit {
    /// A comment's mark: a count pill, or the edge of a region's frame.
    Comment {
        /// The comment the mark stands for.
        annotation: AnnotationId,
    },
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

/// What is under `screen`. Chrome comes first (comment marks, group titles,
/// then the selection's resize handles, then edge anchors), then the bodies
/// as [`body_at`] orders them.
pub fn hit_test(app: &App, screen: Vec2) -> Hit {
    let camera = &app.session.camera;
    let document = &app.document;

    if let Some(annotation) = comment::mark_at(app, screen) {
        return Hit::Comment { annotation };
    }

    let label = document.entities().rev().find(|entity| {
        group_label_rect(entity, ScreenRect::of(camera, entity.rect), camera.zoom)
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
        | Hit::Comment { .. }
        | Hit::Edge { .. }
        | Hit::Empty => None,
    }
}

/// The body under `screen`, ignoring chrome: entities and edges in stack
/// order from the front, then groups, the innermost first. Groups come last
/// whatever their place in the order, because they are containers and a
/// member inside one must be reachable.
pub(crate) fn body_at(app: &App, screen: Vec2) -> Hit {
    body_among(app, screen, true)
}

/// The entity a drag from `screen` would move if no edge were in the way: a
/// page, or a body that is not a group's interior. An edge that crosses an
/// entity leaves it reachable this way.
pub(crate) fn entity_under_edges(app: &App, screen: Vec2) -> Option<EntityId> {
    match body_among(app, screen, false) {
        Hit::PageContent { page: entity, .. } => Some(entity),
        Hit::EntityBody { entity } => {
            let grouped = app
                .document
                .entity(&entity)
                .is_some_and(crate::scope::is_group);
            (!grouped).then_some(entity)
        }
        Hit::Comment { .. }
        | Hit::GroupLabel { .. }
        | Hit::Handle { .. }
        | Hit::Anchor { .. }
        | Hit::GroupBorder { .. }
        | Hit::Edge { .. }
        | Hit::Empty => None,
    }
}

fn body_among(app: &App, screen: Vec2, edges: bool) -> Hit {
    let camera = &app.session.camera;
    let document = &app.document;
    let mut groups = Vec::new();
    for item in document.order().iter().rev() {
        match item {
            ItemId::Edge(id) => {
                if edges
                    && app
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
                // An entity scrolled out of its page cannot be pressed.
                let Some(seen) = crate::seen(app, entity) else {
                    continue;
                };
                let shown = seen.entity;
                let rect = ScreenRect::of(
                    camera,
                    crate::hittable_rect(app, entity).unwrap_or(shown.rect),
                );
                let inside = match &entity.kind {
                    Kind::Group(_) => {
                        groups.push((id, rect));
                        continue;
                    }
                    Kind::Drawing(stored) => {
                        // The ink is hit where the page's scroll has put it.
                        let ink = if let Kind::Drawing(ink) = &shown.kind {
                            ink
                        } else {
                            stored
                        };
                        drawing_rect(rect).contains(screen) && on_drawing(app, id, ink, screen)
                    }
                    Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Shape(_) => {
                        rect.contains(screen)
                    }
                };
                if inside {
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

/// Whether `screen`, already inside the drawing's box, is on the drawing.
/// An unselected drawing is hit on its ink only, so what shows through the
/// empty part of its box can be reached. Once selected the whole box is the
/// drawing, as its outline shows, so it can be dragged from anywhere in it.
fn on_drawing(app: &App, id: &EntityId, drawing: &Drawing, screen: Vec2) -> bool {
    let selected = (app.session.selection).contains(&ItemId::Entity(id.clone()));
    if selected
        || drawing
            .strokes
            .iter()
            .all(|stroke| stroke.points.is_empty())
    {
        return true;
    }
    let camera = &app.session.camera;
    drawing.strokes.iter().any(|stroke| {
        let reach = (stroke.width as f32 * INK_SCALE * camera.zoom).max(DRAWING_MIN_HIT) / 2.0;
        let on_screen = |point: &specular_doc::Point| {
            camera.world_to_screen(Vec2::new(point.x as f32, point.y as f32))
        };
        let mut points = stroke.points.iter().map(on_screen);
        let Some(mut from) = points.next() else {
            return false;
        };
        // A dot is one point, and has no segment.
        let mut nearest = screen.distance(from);
        for to in points {
            nearest = nearest.min(distance_to_segment(screen, from, to));
            from = to;
        }
        nearest <= reach
    })
}

fn drawing_rect(rect: ScreenRect) -> ScreenRect {
    let size = rect.size.max(Vec2::splat(DRAWING_MIN_HIT));
    ScreenRect {
        min: rect.min - (size - rect.size) / 2.0,
        size,
    }
}

/// The title box of a labelled group whose rect is `rect` on screen. It is
/// drawn no wider than the group, so it is hit no wider.
fn group_label_rect(entity: &Entity, rect: ScreenRect, zoom: f32) -> Option<ScreenRect> {
    let label = match &entity.kind {
        Kind::Group(_) => entity.label.as_deref().filter(|label| !label.is_empty())?,
        Kind::Page(_) | Kind::Text(_) | Kind::File(_) | Kind::Drawing(_) | Kind::Shape(_) => {
            return None;
        }
    };
    let scale = title_scale(zoom);
    let width = label.chars().count() as f32 * GROUP_LABEL_CHAR_WIDTH * scale;
    let height = GROUP_LABEL_HEIGHT * scale;
    Some(ScreenRect::new(
        rect.min.x,
        rect.min.y - height,
        width.min(rect.size.x),
        height,
    ))
}
