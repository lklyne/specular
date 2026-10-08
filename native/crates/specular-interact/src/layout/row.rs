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

#[cfg(test)]
mod tests {
    use super::*;

    type Boxes = Vec<(EntityId, Rect)>;
    /// The axis, order and gap a detection found.
    type Found = Option<(Axis, Vec<&'static str>, f64)>;
    /// Name, boxes, tolerance, what is found.
    type Detected = (&'static str, Boxes, f64, Found);
    /// Where every box lands, in order.
    type Landing = Vec<(&'static str, DVec2)>;
    /// Boxes, the box moved, its slot, where every box lands.
    type Moved = (Boxes, &'static str, usize, Landing);

    fn boxed(id: &str, x: f64, y: f64, width: f64) -> (EntityId, Rect) {
        (EntityId::new(id), Rect::new(x, y, width, 80.0))
    }

    /// `count` boxes `width` wide, `gap` apart, named from `a`.
    fn row(width: f64, gap: f64, count: usize) -> Vec<(EntityId, Rect)> {
        (0..count)
            .map(|i| {
                let id = char::from(b'a' + i as u8).to_string();
                boxed(&id, i as f64 * (width + gap), 0.0, width)
            })
            .collect()
    }

    fn ids(row: &Row) -> Vec<&str> {
        row.boxes.iter().map(|(id, _)| id.as_str()).collect()
    }

    /// Electron's unit cases, `tests/unit/reorder-row.test.ts`.
    #[test]
    fn an_even_line_without_overlap_is_a_row() {
        let widened = |by: f64| {
            let mut boxes = row(100.0, 20.0, 3);
            boxes[2].1.x += by;
            boxes
        };
        let column: Vec<(EntityId, Rect)> = (0..3)
            .map(|i| {
                let id = char::from(b'a' + i as u8).to_string();
                (
                    EntityId::new(id),
                    Rect::new(0.0, f64::from(i) * 130.0, 100.0, 80.0),
                )
            })
            .collect();
        let rows: [Detected; 9] = [
            (
                "four even",
                row(100.0, 20.0, 4),
                1.0,
                Some((Axis::X, vec!["a", "b", "c", "d"], 20.0)),
            ),
            ("one gap wider", widened(30.0), 1.0, None),
            (
                "overlapping",
                vec![boxed("a", 0.0, 0.0, 100.0), boxed("b", 50.0, 0.0, 100.0)],
                1.0,
                None,
            ),
            ("none", Vec::new(), 1.0, None),
            ("one", row(100.0, 20.0, 1), 1.0, None),
            (
                "two",
                row(100.0, 20.0, 2),
                1.0,
                Some((Axis::X, vec!["a", "b"], 20.0)),
            ),
            (
                "a column",
                column,
                1.0,
                Some((Axis::Y, vec!["a", "b", "c"], 50.0)),
            ),
            (
                "a unit off, within a unit",
                widened(1.0),
                1.0,
                Some((Axis::X, vec!["a", "b", "c"], 20.5)),
            ),
            ("a unit off, within half", widened(1.0), 0.5, None),
        ];
        for (name, boxes, tolerance, want) in rows {
            let found = Row::detect(&boxes, tolerance);
            let got = found.as_ref().map(|row| (row.axis, ids(row), row.gap));
            assert_eq!(got, want, "{name}");
            if let Some(row) = found {
                assert_eq!(row.origin, DVec2::ZERO, "{name}");
            }
        }
    }

    #[test]
    fn a_box_swaps_half_way_to_the_next_slot() {
        // Centres 50, 170 and 290, so the boundaries are 110 and 230.
        let row = Row::detect(&row(100.0, 20.0, 3), 1.0).unwrap_or_else(|| panic!("a row"));
        for (cursor, slot) in [
            (0.0, 0),
            (105.0, 0),
            (115.0, 1),
            (170.0, 1),
            (200.0, 1),
            (1000.0, 2),
        ] {
            assert_eq!(row.drop_index(cursor), slot, "cursor at {cursor}");
        }
    }

    #[test]
    fn a_reorder_repacks_from_the_origin_and_keeps_each_box_across_the_axis() {
        let at = |x: f64, y: f64| DVec2::new(x, y);
        let rows: [Moved; 3] = [
            // Mixed widths, 10 apart: c to the front.
            (
                vec![
                    boxed("a", 0.0, 0.0, 100.0),
                    boxed("b", 110.0, 0.0, 200.0),
                    boxed("c", 320.0, 0.0, 50.0),
                ],
                "c",
                0,
                vec![
                    ("c", at(0.0, 0.0)),
                    ("a", at(60.0, 0.0)),
                    ("b", at(170.0, 0.0)),
                ],
            ),
            // b sits lower and stays lower.
            (
                vec![
                    boxed("a", 0.0, 0.0, 100.0),
                    boxed("b", 120.0, 40.0, 100.0),
                    boxed("c", 240.0, 0.0, 100.0),
                ],
                "c",
                1,
                vec![
                    ("a", at(0.0, 0.0)),
                    ("c", at(120.0, 0.0)),
                    ("b", at(240.0, 40.0)),
                ],
            ),
            // Dropped where it was: nothing moves.
            (
                row(100.0, 20.0, 3),
                "b",
                1,
                vec![
                    ("a", at(0.0, 0.0)),
                    ("b", at(120.0, 0.0)),
                    ("c", at(240.0, 0.0)),
                ],
            ),
        ];
        for (boxes, moving, to, want) in rows {
            let row = Row::detect(&boxes, 1.0).unwrap_or_else(|| panic!("a row"));
            let landed = row.reordered(&EntityId::new(moving), to);
            let got: Landing = (landed.iter().zip(&want))
                .map(|((_, at), (name, _))| (*name, *at))
                .collect();
            let order: Vec<&str> = landed.iter().map(|(id, _)| id.as_str()).collect();
            assert_eq!(order, want.iter().map(|(id, _)| *id).collect::<Vec<_>>());
            assert_eq!(got, want, "{moving} to {to}");
        }
    }
}
