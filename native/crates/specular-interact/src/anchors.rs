//! Edge anchors: the four dots, one per side, an entity offers for starting
//! an edge. Hit-testing and the scene both read the set from here, so what
//! is drawn is what can be grabbed.

use glam::Vec2;
use specular_doc::{EdgeSide, Entity, EntityId};

use crate::edge_path::{anchor_point, hit_scale, outward, side_point};
use crate::geometry::ScreenRect;
use crate::{App, Gesture, caps};

/// An anchor's hit box: this long along its side and this deep, starting
/// this far outside the entity. The first two follow [`hit_scale`].
const ALONG: f32 = 68.0;
const ACROSS: f32 = 32.0;
const GAP: f32 = 4.0;

/// Every side of an entity, in the order they are tested.
pub(crate) const SIDES: [EdgeSide; 4] = [
    EdgeSide::Top,
    EdgeSide::Right,
    EdgeSide::Bottom,
    EdgeSide::Left,
];

/// One anchor on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    /// The entity the anchor belongs to.
    pub entity: EntityId,
    /// The side it is on.
    pub side: EdgeSide,
    /// The centre of its dot, in logical screen pixels.
    pub point: Vec2,
    /// Whether the dot is shown: the pointer is over the anchor's hit box,
    /// or an edge is being dragged, when every anchor is a target.
    pub active: bool,
}

impl App {
    /// The anchors to draw, in logical screen pixels.
    ///
    /// The hovered entity and the selected one, when it is the whole
    /// selection, offer all four; only the one the pointer is over is
    /// `active`. While an edge is dragged every entity that can be connected
    /// offers all four, all active; the scene leaves out those off screen. Nothing is offered under any
    /// other gesture or while text is edited.
    pub fn anchors(&self) -> Vec<Anchor> {
        let dragging = matches!(self.session.gesture, Some(Gesture::EdgeDrag(_)));
        if self.session.gesture.is_some() && !dragging || self.session.editing.is_some() {
            return Vec::new();
        }
        let over = self.session.pointer.and_then(|pointer| at(self, pointer));
        let entities: Vec<&Entity> = if dragging {
            (self.document.entities())
                .filter(|entity| caps::has_anchors(&entity.kind))
                .collect()
        } else {
            eligible(self).collect()
        };
        entities
            .into_iter()
            .filter_map(|entity| Some((entity, self.shown_on_screen(entity)?)))
            .flat_map(|(entity, rect)| {
                SIDES.map(|side| Anchor {
                    entity: entity.id.clone(),
                    side,
                    point: anchor_point(rect, side),
                    active: dragging || over.as_ref() == Some(&(entity.id.clone(), side)),
                })
            })
            .collect()
    }
}

/// The entities that offer anchors: the hovered one and the selected one,
/// when nothing else is selected with it. Starting an edge is a one-entity
/// affordance, and the hovered entity is how the end of an existing edge is
/// reached without selecting first.
pub(crate) fn eligible(app: &App) -> impl Iterator<Item = &Entity> {
    let selection = &app.session.selection;
    let selected = selection.single_entity();
    let hovered = app
        .session
        .hover
        .as_ref()
        .filter(|id| Some(*id) != selected);
    [selected, hovered]
        .into_iter()
        .flatten()
        .filter(|_| selection.items().len() <= 1)
        .filter_map(|id| app.document.entity(id))
        .filter(|entity| caps::has_anchors(&entity.kind))
}

/// The anchor under `screen`, if any.
pub(crate) fn at(app: &App, screen: Vec2) -> Option<(EntityId, EdgeSide)> {
    let camera = &app.session.camera;
    eligible(app).find_map(|entity| {
        let rect = app.shown_on_screen(entity)?;
        let side = SIDES
            .into_iter()
            .find(|side| hit_rect(rect, *side, camera.zoom).contains(screen))?;
        Some((entity.id.clone(), side))
    })
}

/// The hit box of the anchor on `side` of an entity whose rect is `rect` on
/// screen: centred on the side, just outside it.
pub(crate) fn hit_rect(rect: ScreenRect, side: EdgeSide, zoom: f32) -> ScreenRect {
    let scale = hit_scale(zoom);
    let (along, across) = (ALONG * scale, ACROSS * scale);
    let out = outward(side);
    let size = if out.x == 0.0 {
        Vec2::new(along, across)
    } else {
        Vec2::new(across, along)
    };
    let centre = side_point(rect, side) + out * (GAP + across / 2.0);
    ScreenRect {
        min: centre - size / 2.0,
        size,
    }
}
