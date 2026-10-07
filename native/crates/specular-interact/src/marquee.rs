//! Marquee selection: which items a dragged rect takes, and what releasing
//! the drag does to the selection.
//!
//! The entities the rect touches resolve through two promotions, so a
//! marquee never takes a partial slice of a group:
//!
//! - A group the rect fully encloses is one unit. Its touched descendants
//!   collapse into it, and nested enclosed groups collapse to the outermost.
//!   Enclosure is the only way to take a group with no touched children.
//! - The deepest group holding every unit is the scope. Units directly in
//!   the scope stay as they are; any other is replaced by its ancestor that
//!   is. So a marquee inside one group picks its children one by one, and a
//!   marquee that also touches something outside picks the whole group.

use glam::{DVec2, Vec2};
use specular_core::Modifiers;
use specular_doc::{Document, EdgeSide, Entity, EntityId, ItemId, Rect};

use crate::scope::is_group;
use crate::{App, Gesture, PointerInput, geometry, select};

/// How far the pointer must travel on either axis, in logical pixels, before
/// a press becomes a drag.
pub(crate) const DRAG_THRESHOLD: f32 = 4.0;

/// Which entities a marquee takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MarqueeMode {
    /// Everything the rect overlaps.
    #[default]
    Intersect,
    /// Only what the rect fully encloses. Command or Control held.
    Contain,
}

impl MarqueeMode {
    pub(crate) const fn held(modifiers: Modifiers) -> Self {
        if modifiers.meta || modifiers.control {
            Self::Contain
        } else {
            Self::Intersect
        }
    }
}

impl App {
    /// The marquee being dragged, in canvas space. `None` until the press
    /// has travelled far enough to be a drag.
    pub fn marquee(&self) -> Option<Rect> {
        match &self.session.gesture {
            Some(Gesture::Marquee {
                start,
                current,
                dragged: true,
                ..
            }) => Some(geometry::spanning(*start, *current)),
            Some(
                Gesture::Marquee { .. }
                | Gesture::Move(_)
                | Gesture::Resize(_)
                | Gesture::CommentRegion { .. }
                | Gesture::Place(_)
                | Gesture::Draw(_),
            )
            | None => None,
        }
    }

    /// What releasing the marquee now would take, for outlining while it is
    /// dragged. Entities come first, back-to-front, then edges.
    pub fn marquee_items(&self) -> Vec<ItemId> {
        let (Some(rect), Some(Gesture::Marquee { origin, mode, .. })) =
            (self.marquee(), &self.session.gesture)
        else {
            return Vec::new();
        };
        items_in(&self.document, rect, *mode, origin.as_ref())
    }
}

/// The pointer moved to `screen` with a marquee in flight.
pub(crate) fn drag(app: &mut App, input: &PointerInput) {
    let world = app.session.camera.screen_to_world(input.screen).as_dvec2();
    if let Some(Gesture::Marquee {
        start_screen,
        current,
        dragged,
        mode,
        ..
    }) = &mut app.session.gesture
    {
        let travel = (input.screen - *start_screen).abs();
        *dragged |= travel.max_element() >= DRAG_THRESHOLD;
        *current = world;
        *mode = MarqueeMode::held(input.modifiers);
    }
}

/// The button came up on a marquee from `start` that began on `origin`.
///
/// A press that never became a drag is a click: on `origin` when there is
/// one, otherwise on empty canvas. A drag selects what the rect takes, or
/// toggles it with Shift, Command or Control held.
pub(crate) fn finish(
    app: &mut App,
    start: DVec2,
    start_screen: Vec2,
    origin: Option<EntityId>,
    dragged: bool,
    input: &PointerInput,
) {
    let additive = select::is_additive(input.modifiers);
    if !dragged {
        match origin {
            Some(entity) if additive && !is_group_id(&app.document, &entity) => {
                app.session.selection.toggle(ItemId::Entity(entity));
            }
            Some(entity) => app.session.selection.set([ItemId::Entity(entity)]),
            None => clear_unless(app, additive),
        }
        return;
    }
    if (input.screen - start_screen).abs().min_element() < DRAG_THRESHOLD {
        clear_unless(app, additive);
        return;
    }
    let end = app.session.camera.screen_to_world(input.screen).as_dvec2();
    let items = items_in(
        &app.document,
        geometry::spanning(start, end),
        MarqueeMode::held(input.modifiers),
        origin.as_ref(),
    );
    if additive {
        for item in items {
            app.session.selection.toggle(item);
        }
    } else {
        app.session.selection.set(items);
    }
}

/// A click on empty canvas clears the selection, unless a modifier that
/// extends it is held.
fn clear_unless(app: &mut App, additive: bool) {
    if !additive {
        app.session.selection.set([]);
    }
}

fn is_group_id(document: &Document, id: &EntityId) -> bool {
    document.entity(id).is_some_and(is_group)
}

/// The items a marquee over `rect` takes: entities back-to-front, then
/// edges. `excluded` is the entity the drag began on, which a marquee
/// started through a body leaves out.
pub(crate) fn items_in(
    document: &Document,
    rect: Rect,
    mode: MarqueeMode,
    excluded: Option<&EntityId>,
) -> Vec<ItemId> {
    let entities = entities_in(document, rect, mode, excluded);
    let edges = document
        .edges()
        .filter(|edge| {
            let end = |id, side| Some(anchor_point(document.entity(id)?.rect, side));
            let (Some(from), Some(to)) =
                (end(&edge.from, edge.from_side), end(&edge.to, edge.to_side))
            else {
                return false;
            };
            match mode {
                MarqueeMode::Contain => holds_point(rect, from) && holds_point(rect, to),
                MarqueeMode::Intersect => segment_crosses(from, to, rect),
            }
        })
        .map(|edge| ItemId::Edge(edge.id.clone()));
    entities
        .into_iter()
        .map(ItemId::Entity)
        .chain(edges)
        .collect()
}

fn entities_in(
    document: &Document,
    rect: Rect,
    mode: MarqueeMode,
    excluded: Option<&EntityId>,
) -> Vec<EntityId> {
    let included = |entity: &&Entity| Some(&entity.id) != excluded;
    // Outermost first.
    let chain = |id: &EntityId| -> Vec<&EntityId> {
        let mut chain: Vec<&EntityId> = document.ancestors(id).map(|group| &group.id).collect();
        chain.reverse();
        chain
    };
    let enclosed: Vec<&EntityId> = document
        .entities()
        .filter(included)
        .filter(|entity| is_group(entity) && encloses(rect, entity.rect))
        .map(|entity| &entity.id)
        .collect();
    let touched = document
        .entities()
        .filter(included)
        .filter(|entity| {
            !is_group(entity)
                && match mode {
                    MarqueeMode::Contain => encloses(rect, entity.rect),
                    MarqueeMode::Intersect => overlaps(rect, entity.rect),
                }
        })
        .map(|entity| &entity.id);

    let mut units: Vec<&EntityId> = Vec::new();
    for id in touched.chain(enclosed.iter().copied()) {
        let unit = chain(id)
            .into_iter()
            .find(|group| enclosed.contains(group))
            .unwrap_or(id);
        if !units.contains(&unit) {
            units.push(unit);
        }
    }
    let chains: Vec<Vec<&EntityId>> = units.iter().map(|unit| chain(unit)).collect();
    let Some(first) = chains.first() else {
        return Vec::new();
    };
    // Lineages are shared exactly as far as their chains agree, so the scope
    // is the common leading run.
    let depth = chains
        .iter()
        .map(|chain| chain.iter().zip(first).take_while(|(a, b)| a == b).count())
        .min()
        .unwrap_or(0);
    let resolved: Vec<&EntityId> = units
        .iter()
        .zip(&chains)
        .map(|(unit, chain)| chain.get(depth).copied().unwrap_or(unit))
        .collect();
    document
        .entities()
        .filter(|entity| resolved.contains(&&entity.id))
        .map(|entity| entity.id.clone())
        .collect()
}

/// Whether the rects share any area. Touching edges do not count.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

/// Whether `inner` lies wholly inside `outer`, edges included.
fn encloses(outer: Rect, inner: Rect) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && outer.x + outer.width >= inner.x + inner.width
        && outer.y + outer.height >= inner.y + inner.height
}

fn holds_point(rect: Rect, point: DVec2) -> bool {
    encloses(rect, Rect::new(point.x, point.y, 0.0, 0.0))
}

/// Where an edge meets an entity for the marquee test: the middle of the
/// side it names, or the entity's centre when it names none.
fn anchor_point(rect: Rect, side: Option<EdgeSide>) -> DVec2 {
    let (origin, size) = (geometry::origin(rect), geometry::size(rect));
    let unit = match side {
        Some(EdgeSide::Top) => DVec2::new(0.5, 0.0),
        Some(EdgeSide::Bottom) => DVec2::new(0.5, 1.0),
        Some(EdgeSide::Left) => DVec2::new(0.0, 0.5),
        Some(EdgeSide::Right) => DVec2::new(1.0, 0.5),
        None => DVec2::splat(0.5),
    };
    origin + size * unit
}

/// Whether the segment from `a` to `b` crosses or lies inside `rect`
/// (Liang-Barsky clipping).
fn segment_crosses(a: DVec2, b: DVec2, rect: Rect) -> bool {
    let delta = b - a;
    let low = geometry::origin(rect);
    let high = low + geometry::size(rect);
    let sides = [
        (-delta.x, a.x - low.x),
        (delta.x, high.x - a.x),
        (-delta.y, a.y - low.y),
        (delta.y, high.y - a.y),
    ];
    let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
    for (direction, distance) in sides {
        if direction == 0.0 {
            if distance < 0.0 {
                return false;
            }
        } else if direction < 0.0 {
            enter = enter.max(distance / direction);
        } else {
            leave = leave.min(distance / direction);
        }
        if enter > leave {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECT: Rect = Rect::new(0.0, 0.0, 100.0, 100.0);

    #[test]
    fn a_segment_through_the_rect_crosses_it_without_an_end_inside() {
        assert!(segment_crosses(
            DVec2::new(-50.0, 50.0),
            DVec2::new(150.0, 50.0),
            RECT
        ));
    }

    #[test]
    fn a_segment_wholly_inside_counts() {
        assert!(segment_crosses(
            DVec2::new(10.0, 10.0),
            DVec2::new(20.0, 30.0),
            RECT
        ));
    }

    #[test]
    fn a_segment_passing_a_corner_outside_misses() {
        assert!(!segment_crosses(
            DVec2::new(-50.0, 20.0),
            DVec2::new(20.0, -50.0),
            RECT
        ));
    }

    #[test]
    fn a_segment_parallel_to_a_side_and_outside_misses() {
        assert!(!segment_crosses(
            DVec2::new(-10.0, 0.0),
            DVec2::new(-10.0, 100.0),
            RECT
        ));
    }
}
