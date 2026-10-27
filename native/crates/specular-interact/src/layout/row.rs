//! A reorderable row (ADR 0015 D7): boxes that already read as an evenly
//! spaced line, the slot a cursor is over, and the line repacked with one
//! box moved. Geometry is the truth: the order is read off the boxes each
//! time and nothing here is kept.

use glam::DVec2;
use specular_doc::{EntityId, Rect};

use super::Axis;

/// How far the gaps of a loose selection may differ and still read as
/// even, in canvas units. Wider than a managed line needs, so a row that
/// was arranged still counts after rounding.
pub(crate) const SELECTION_GAP_TOLERANCE: f64 = 4.0;

/// An evenly spaced line of boxes, frozen as it was found.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Row {
    pub(crate) axis: Axis,
    /// The boxes in order along the axis.
    pub(crate) boxes: Vec<(EntityId, Rect)>,
    /// The gap between neighbours: the mean of the gaps found.
    pub(crate) gap: f64,
    /// The least corner of the boxes, where packing starts.
    pub(crate) origin: DVec2,
}

fn spread(values: impl Iterator<Item = f64> + Clone) -> f64 {
    values.clone().fold(f64::NEG_INFINITY, f64::max) - values.fold(f64::INFINITY, f64::min)
}

/// The axis the centres of `rects` are spread further along. A tie is a
/// row.
pub(crate) fn dominant_axis(rects: &[Rect]) -> Axis {
    let along = |axis: Axis| spread(rects.iter().map(move |rect| axis.centre(*rect)));
    if along(Axis::X) >= along(Axis::Y) {
        Axis::X
    } else {
        Axis::Y
    }
}

impl Row {
    /// The row `boxes` make, or `None` when they are fewer than two, overlap
    /// along their axis, or their gaps differ by more than `tolerance`.
    pub(crate) fn detect(boxes: &[(EntityId, Rect)], tolerance: f64) -> Option<Self> {
        if boxes.len() < 2 {
            return None;
        }
        let rects: Vec<Rect> = boxes.iter().map(|(_, rect)| *rect).collect();
        let axis = dominant_axis(&rects);
        let mut sorted = boxes.to_vec();
        sorted.sort_by(|a, b| axis.lead(a.1).total_cmp(&axis.lead(b.1)));
        let gaps: Vec<f64> = (sorted.windows(2))
            .map(|pair| axis.lead(pair[1].1) - axis.trail(pair[0].1))
            .collect();
        if gaps.iter().any(|gap| *gap < 0.0) || spread(gaps.iter().copied()) > tolerance {
            return None;
        }
        let least = |axis: Axis| {
            (rects.iter())
                .map(|rect| axis.lead(*rect))
                .fold(f64::INFINITY, f64::min)
        };
        Some(Self {
            axis,
            gap: gaps.iter().sum::<f64>() / gaps.len() as f64,
            origin: DVec2::new(least(Axis::X), least(Axis::Y)),
            boxes: sorted,
        })
    }

    /// A managed line as a row: its members in layout order at its gap.
    pub(crate) fn of_line(line: &super::Line) -> Option<Self> {
        if line.members.len() < 2 {
            return None;
        }
        let least = |axis: Axis| {
            (line.members.iter())
                .map(|(_, rect)| axis.lead(*rect))
                .fold(f64::INFINITY, f64::min)
        };
        Some(Self {
            axis: line.axis,
            gap: line.gap,
            origin: DVec2::new(least(Axis::X), least(Axis::Y)),
            boxes: line.members.clone(),
        })
    }

    /// Where `id` is in the row.
    pub(crate) fn index_of(&self, id: &EntityId) -> Option<usize> {
        self.boxes.iter().position(|(member, _)| member == id)
    }

    /// The slot a cursor at `along` the axis is over. The boundaries are
    /// the midpoints between neighbouring centres, so a box swaps once the
    /// cursor is half way to the next slot, and a cursor resting on a box's
    /// own centre holds its slot.
    pub(crate) fn drop_index(&self, along: f64) -> usize {
        (self.boxes.windows(2))
            .filter(|pair| {
                along > f64::midpoint(self.axis.centre(pair[0].1), self.axis.centre(pair[1].1))
            })
            .count()
    }

    /// The row's order with `moving` at slot `to`.
    fn order_with(&self, moving: &EntityId, to: usize) -> Vec<&(EntityId, Rect)> {
        let mut order: Vec<&(EntityId, Rect)> =
            (self.boxes.iter()).filter(|(id, _)| id != moving).collect();
        if let Some(found) = self.boxes.iter().find(|(id, _)| id == moving) {
            order.insert(to.min(order.len()), found);
        }
        order
    }

    /// Where every box sits with `moving` at slot `to`: packed from the
    /// origin at the row's gap, each keeping its own place across the axis.
    pub(crate) fn reordered(&self, moving: &EntityId, to: usize) -> Vec<(EntityId, DVec2)> {
        self.packed_at(&self.order_with(moving, to), self.gap)
    }

    /// Where every box sits with the gap changed to `gap`, in the order it
    /// has.
    pub(crate) fn regapped(&self, gap: f64) -> Vec<(EntityId, DVec2)> {
        self.packed_at(&self.boxes.iter().collect::<Vec<_>>(), gap)
    }

    fn packed_at(&self, order: &[&(EntityId, Rect)], gap: f64) -> Vec<(EntityId, DVec2)> {
        let across = self.axis.other();
        let mut cursor = self.axis.of(self.origin);
        order
            .iter()
            .map(|(id, rect)| {
                let at = self.axis.point(cursor, across.lead(*rect));
                cursor += self.axis.size(*rect) + gap;
                (id.clone(), at)
            })
            .collect()
    }
}
