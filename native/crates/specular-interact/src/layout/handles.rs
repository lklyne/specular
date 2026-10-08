//! The handles of a line: a reorder dot at the middle of each box and a gap
//! strip between each pair of neighbours (ADR 0015).
//!
//! A line is shown two ways. A managed row or column shows them when it or
//! one of its members is selected. A loose selection shows them when it
//! already reads as an even row. Hit-testing and drawing both read these
//! lists, so what is seen is what can be grabbed.

use glam::Vec2;
use specular_doc::{EntityId, Rect};

use super::row::{Row, SELECTION_GAP_TOLERANCE};
use super::{Axis, group_of, line, line_axis};
use crate::{App, Gesture, ScreenRect, Tool};

/// A dot's radius at rest, in screen pixels.
pub const DOT_RADIUS: f32 = 4.0;
/// The side of the square a dot is grabbed in, where the box is big enough.
const DOT_HIT: f32 = 14.0;
/// The most of a box's shorter side that square takes.
const DOT_HIT_FRACTION: f32 = 0.4;
/// The least thickness a gap strip is grabbed in, however close the boxes.
const GAP_MIN_HIT: f32 = 10.0;

/// A handle of a line.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LayoutHandle {
    /// The dot that drags a box to another slot.
    Reorder {
        /// The box.
        entity: EntityId,
    },
    /// The strip that drags the gap.
    Gap {
        /// The managed group whose gap it is, or `None` for a loose
        /// selection's.
        group: Option<EntityId>,
        /// The axis the line is packed along.
        axis: Axis,
    },
}

/// A reorder dot.
#[derive(Debug, Clone, PartialEq)]
pub struct ReorderDot {
    /// The box it reorders.
    pub entity: EntityId,
    /// Its middle, on screen.
    pub centre: Vec2,
    /// The side of the square it is grabbed in, which is also how wide it
    /// draws under the pointer.
    pub hit: f32,
    /// Whether the pointer is on it.
    pub hovered: bool,
}

/// A gap strip.
#[derive(Debug, Clone, PartialEq)]
pub struct GapHandle {
    /// The managed group whose gap it is, or `None` for a loose selection's.
    pub group: Option<EntityId>,
    /// The axis the line is packed along.
    pub axis: Axis,
    /// The strip on screen: the gap along the axis, the whole line across
    /// it.
    pub rect: ScreenRect,
    /// Whether the pointer is on it.
    pub hovered: bool,
}

/// The lines that show handles now, each with the managed group it is.
pub(crate) fn lines(app: &App) -> Vec<(Option<EntityId>, Row)> {
    let document = &app.document;
    let selected: Vec<&EntityId> = app.session.selection.entities().collect();
    if selected.is_empty() || app.session.tool != Tool::Select {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut managed_members: Vec<EntityId> = Vec::new();
    for entity in document.entities() {
        if group_of(entity).and_then(line_axis).is_none() {
            continue;
        }
        let Some(line) = line(document, &entity.id) else {
            continue;
        };
        let shown = selected.contains(&&entity.id)
            || line.members.iter().any(|(id, _)| selected.contains(&id));
        managed_members.extend(line.members.iter().map(|(id, _)| id.clone()));
        if shown && let Some(row) = Row::of_line(&line) {
            out.push((Some(entity.id.clone()), row));
        }
    }
    let loose: Vec<(EntityId, Rect)> = (selected.iter())
        .filter(|id| !managed_members.contains(id))
        .filter_map(|id| document.entity(id))
        .filter(|entity| group_of(entity).is_none())
        .map(|entity| (entity.id.clone(), entity.rect))
        .collect();
    if let Some(row) = Row::detect(&loose, SELECTION_GAP_TOLERANCE) {
        out.push((None, row));
    }
    out
}

/// Whether the handles are up: at rest, and during a drag of one of them.
fn shown(app: &App) -> bool {
    match &app.session.gesture {
        None | Some(Gesture::Line(_)) => true,
        Some(
            Gesture::Move(_)
            | Gesture::Resize(_)
            | Gesture::Marquee { .. }
            | Gesture::Comment(_)
            | Gesture::Place(_)
            | Gesture::Draw(_)
            | Gesture::TextSelect(_)
            | Gesture::EdgeDrag(_),
        ) => false,
    }
}

fn square(dot: &ReorderDot) -> ScreenRect {
    ScreenRect::square(dot.centre, dot.hit)
}

impl App {
    /// The reorder dots to draw and to grab, without the one being dragged.
    pub fn reorder_dots(&self) -> Vec<ReorderDot> {
        if !shown(self) || self.session.editing.is_some() {
            return Vec::new();
        }
        let camera = &self.session.camera;
        let moving = self.line_drag().and_then(|drag| drag.moving());
        let resting = self.session.gesture.is_none();
        (lines(self).into_iter())
            .flat_map(|(_, row)| row.boxes)
            .filter(|(id, _)| moving != Some(id))
            .map(|(entity, rect)| {
                let on_screen = ScreenRect::of(camera, rect);
                let shortest = on_screen.size.min_element();
                let mut dot = ReorderDot {
                    entity,
                    centre: on_screen.centre(),
                    hit: DOT_HIT.min(shortest * DOT_HIT_FRACTION),
                    hovered: false,
                };
                dot.hovered =
                    resting && (self.session.pointer).is_some_and(|at| square(&dot).contains(at));
                dot
            })
            .collect()
    }

    /// The gap strips to draw and to grab.
    pub fn gap_handles(&self) -> Vec<GapHandle> {
        if !shown(self) || self.session.editing.is_some() {
            return Vec::new();
        }
        let camera = &self.session.camera;
        let resting = self.session.gesture.is_none();
        let over_dot = resting
            && (self.session.pointer).is_some_and(|at| {
                self.reorder_dots()
                    .iter()
                    .any(|dot| dot.hovered && square(dot).contains(at))
            });
        let mut out = Vec::new();
        for (group, row) in lines(self) {
            let axis = row.axis;
            let across = axis.other();
            let mut boxes: Vec<ScreenRect> = (row.boxes.iter())
                .map(|(_, rect)| ScreenRect::of(camera, *rect))
                .collect();
            let lead = |rect: &ScreenRect| axis.of(rect.min.as_dvec2()) as f32;
            let trail = |rect: &ScreenRect| axis.of(rect.max().as_dvec2()) as f32;
            let low = (boxes.iter())
                .map(|rect| across.of(rect.min.as_dvec2()) as f32)
                .fold(f32::INFINITY, f32::min);
            let high = (boxes.iter())
                .map(|rect| across.of(rect.max().as_dvec2()) as f32)
                .fold(f32::NEG_INFINITY, f32::max);
            // Where the boxes are, not the order they are kept in: a reorder
            // in flight shows the line as it would be.
            boxes.sort_by(|a, b| lead(a).total_cmp(&lead(b)));
            for pair in boxes.windows(2) {
                let (mut start, mut thickness) =
                    (trail(&pair[0]), lead(&pair[1]) - trail(&pair[0]));
                if thickness < GAP_MIN_HIT {
                    start += thickness / 2.0 - GAP_MIN_HIT / 2.0;
                    thickness = GAP_MIN_HIT;
                }
                let min = axis.point(f64::from(start), f64::from(low)).as_vec2();
                let size = axis
                    .point(f64::from(thickness), f64::from(high - low))
                    .as_vec2();
                let rect = ScreenRect { min, size };
                out.push(GapHandle {
                    hovered: resting
                        && !over_dot
                        && (self.session.pointer).is_some_and(|at| rect.contains(at)),
                    group: group.clone(),
                    axis,
                    rect,
                });
            }
        }
        out
    }

    /// The line handle at `screen`: a dot before a strip, where they
    /// overlap.
    pub(crate) fn layout_handle_at(&self, screen: Vec2) -> Option<LayoutHandle> {
        if self.session.gesture.is_some() {
            return None;
        }
        let dot = (self.reorder_dots().into_iter())
            .find(|dot| square(dot).contains(screen))
            .map(|dot| LayoutHandle::Reorder { entity: dot.entity });
        dot.or_else(|| {
            (self.gap_handles().into_iter())
                .find(|strip| strip.rect.contains(screen))
                .map(|strip| LayoutHandle::Gap {
                    group: strip.group,
                    axis: strip.axis,
                })
        })
    }
}
