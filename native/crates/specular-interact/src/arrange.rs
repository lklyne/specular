//! Arranging the selection into a row, a column or a grid: the dock's
//! tidy-up buttons.
//!
//! The arrangement keeps the footprint the cluster already has and only
//! evens the spacing inside it: the first item and the last item's far edge
//! stay where they are, and the gaps between become equal. A fixed gap would
//! throw away the layout the person built. One arrange is one undo step.

use glam::DVec2;
use specular_doc::{Command, EntityId, Rect};

use crate::layout::Axis;
use crate::{App, Effect, anchor, grid, live, scroll_follow::Scrolls, update, verbs};

/// The least gap the arrange leaves between items, so a footprint that is too
/// tight grows instead of overlapping.
const MIN_GAP: f64 = 80.0;

/// How the selection is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArrangeMode {
    /// One horizontal line: even gaps across the x-extent, tops aligned.
    Row,
    /// One vertical line: even gaps across the y-extent, left edges aligned.
    Column,
    /// The two-dimensional structure the items already have, with the gaps
    /// between its rows and between its columns made even.
    Grid,
}

/// One item to arrange: where it is and how big.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Placed {
    /// The entity.
    pub(crate) id: EntityId,
    /// Its rect.
    pub(crate) rect: Rect,
}

fn least(values: impl Iterator<Item = f64>) -> f64 {
    values.fold(f64::INFINITY, f64::min)
}

fn most(values: impl Iterator<Item = f64>) -> f64 {
    values.fold(f64::NEG_INFINITY, f64::max)
}

/// Where each of `boxes` goes: its new top-left corner, in the order given.
/// `None` for fewer than two, which have nothing to arrange.
pub(crate) fn targets(boxes: &[Placed], mode: ArrangeMode) -> Option<Vec<(EntityId, DVec2)>> {
    if boxes.len() < 2 {
        return None;
    }
    let corners = match mode {
        ArrangeMode::Grid => {
            let xs = bands(boxes, Axis::X);
            let ys = bands(boxes, Axis::Y);
            xs.into_iter()
                .zip(ys)
                .map(|(x, y)| DVec2::new(x, y))
                .collect()
        }
        ArrangeMode::Row => line(boxes, Axis::X),
        ArrangeMode::Column => line(boxes, Axis::Y),
    };
    Some(
        (boxes.iter())
            .map(|item| item.id.clone())
            .zip(corners)
            .collect(),
    )
}

/// One line along `axis`: the gaps evened across the cluster's extent, the
/// other axis aligned to the cluster's least edge.
fn line(boxes: &[Placed], axis: Axis) -> Vec<DVec2> {
    let across = axis.other();
    let along = cells(boxes, axis);
    let aligned = grid::snap(least(boxes.iter().map(|item| across.lead(item.rect))));
    along
        .into_iter()
        .map(|lead| match axis {
            Axis::X => DVec2::new(lead, aligned),
            Axis::Y => DVec2::new(aligned, lead),
        })
        .collect()
}

/// The leading edge of every box along `axis` once the gaps between them are
/// even, the first box's edge held in place and the gap floored at
/// [`MIN_GAP`].
fn cells(boxes: &[Placed], axis: Axis) -> Vec<f64> {
    let across = axis.other();
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&a, &b| {
        axis.lead(boxes[a].rect)
            .total_cmp(&axis.lead(boxes[b].rect))
    });
    let (first, last) = (boxes[order[0]].rect, boxes[order[order.len() - 1]].rect);
    let along = axis.trail(last) - axis.lead(first);
    // A cluster collapsed on this axis, a column made a row, has no extent to
    // spread over, so it takes the one it has on the other axis.
    let across_extent = most(boxes.iter().map(|item| across.trail(item.rect)))
        - least(boxes.iter().map(|item| across.lead(item.rect)));
    let extent = along.max(across_extent);
    let total: f64 = boxes.iter().map(|item| axis.size(item.rect)).sum();
    // The gap is snapped once, not each edge, so every pair of neighbours is
    // exactly as far apart as every other.
    let gap = grid::snap(((extent - total) / (boxes.len() - 1) as f64).max(MIN_GAP));
    let mut leads = vec![0.0; boxes.len()];
    let mut cursor = grid::snap(axis.lead(first));
    for index in order {
        leads[index] = cursor;
        cursor += axis.size(boxes[index].rect) + gap;
    }
    leads
}

/// The leading edge of every box along `axis` once the bands it sits in are
/// evenly spaced. A band is a run of boxes that overlap along the axis: a
/// row for `Y`, a column for `X`. Holes stay holes.
fn bands(boxes: &[Placed], axis: Axis) -> Vec<f64> {
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&a, &b| {
        axis.lead(boxes[a].rect)
            .total_cmp(&axis.lead(boxes[b].rect))
    });
    let mut runs: Vec<Vec<usize>> = Vec::new();
    let mut run_end = f64::NEG_INFINITY;
    for index in order {
        let rect = boxes[index].rect;
        match runs.last_mut() {
            Some(run) if axis.lead(rect) < run_end => {
                run.push(index);
                run_end = run_end.max(axis.trail(rect));
            }
            _ => {
                runs.push(vec![index]);
                run_end = axis.trail(rect);
            }
        }
    }
    let mut leads = vec![0.0; boxes.len()];
    if runs.len() < 2 {
        for (index, item) in boxes.iter().enumerate() {
            leads[index] = grid::snap(axis.lead(item.rect));
        }
        return leads;
    }
    let starts: Vec<f64> = (runs.iter())
        .map(|run| least(run.iter().map(|&i| axis.lead(boxes[i].rect))))
        .collect();
    let sizes: Vec<f64> = (runs.iter().zip(&starts))
        .map(|(run, start)| most(run.iter().map(|&i| axis.trail(boxes[i].rect))) - start)
        .collect();
    let last = runs.len() - 1;
    let extent = starts[last] + sizes[last] - starts[0];
    let gap = ((extent - sizes.iter().sum::<f64>()) / last as f64).max(MIN_GAP);
    let mut cursor = starts[0];
    for (run, size) in runs.iter().zip(&sizes) {
        let lead = grid::snap(cursor);
        for &index in run {
            leads[index] = lead;
        }
        cursor = lead + size + gap;
    }
    leads
}

/// Arranges the selected entities, as one undo step. Does nothing for fewer
/// than two, or when every one is already where the arrangement puts it.
pub(crate) fn run(app: &mut App, mode: ArrangeMode, effects: &mut Vec<Effect>) {
    let members = app.selection_scope().members;
    if let Some(step) = arrange_command(app, &members, mode) {
        update::document_step(app, step, effects);
    }
}

/// The step that arranges `entities`, keeping the footprint they have. `None`
/// for fewer than two, or when every one is already where the arrangement
/// puts it.
pub fn arrange_command(app: &App, entities: &[EntityId], mode: ArrangeMode) -> Option<Command> {
    let boxes: Vec<Placed> = (entities.iter())
        .filter_map(|id| app.document.entity(id))
        .map(|entity| Placed {
            id: entity.id.clone(),
            rect: entity.rect,
        })
        .collect();
    place_command(app, &targets(&boxes, mode)?)
}

/// The step that moves each entity to the top-left given, with everything
/// inside its group and hooked to its page, and hooks what moved to the page
/// it lands on. `None` when every one is there already.
pub fn place_command(app: &App, targets: &[(EntityId, DVec2)]) -> Option<Command> {
    let commands: Vec<Command> = (targets.iter())
        .flat_map(|(id, to)| {
            let Some(entity) = app.document.entity(id) else {
                return Vec::new();
            };
            let by = *to - DVec2::new(entity.rect.x, entity.rect.y);
            if by == DVec2::ZERO {
                return Vec::new();
            }
            verbs::move_commands(&app.document, std::slice::from_ref(id), by)
        })
        .collect();
    if commands.is_empty() {
        return None;
    }
    let members: Vec<EntityId> = targets.iter().map(|(id, _)| id.clone()).collect();
    let mut trial = app.document.clone();
    Some(anchor::then_reanchor(
        &mut trial,
        &Scrolls::of(app),
        live::batch(commands),
        &members,
        &app.scope_of(&members).operands,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tight_footprint_grows_rather_than_overlaps() {
        let items: Vec<Placed> = [("a", 0.0), ("b", 35.0), ("c", 70.0)]
            .into_iter()
            .map(|(id, y)| Placed {
                id: EntityId::new(id),
                rect: Rect::new(0.0, y, 80.0, 50.0),
            })
            .collect();
        let moved = targets(&items, ArrangeMode::Column).unwrap_or_default();
        let rects: Vec<Rect> = (moved.iter().zip(&items))
            .map(|((_, to), item)| Rect::new(to.x, to.y, item.rect.width, item.rect.height))
            .collect();
        let [a, b, c] = rects[..] else {
            panic!("three rects");
        };
        assert_eq!(a.y, 0.0);
        let (first, second) = (b.y - (a.y + a.height), c.y - (b.y + b.height));
        assert!(first >= MIN_GAP);
        assert_eq!(first, second);
    }
}
