//! The alignment detector: which edges and centres of the dragged rects sit
//! on a neighbour's.

use specular_doc::Rect;

use super::{AlignmentGuide, GuideAxis, GuideReference, SnapCandidate};

/// Every alignment between a rect of `dragged` and one of `candidates`
/// within `tolerance`. `references` limits which parts of a dragged rect
/// count, as a resize does to the edges its handle moves.
///
/// A centre guide is dropped when both edges either side of it are aligned
/// with the same neighbour, since they already say it.
pub(crate) fn alignment_guides(
    dragged: &[SnapCandidate],
    candidates: &[SnapCandidate],
    references: Option<&[GuideReference]>,
    tolerance: f64,
) -> Vec<AlignmentGuide> {
    let mut guides = Vec::new();
    for moving in dragged {
        for candidate in candidates.iter().filter(|c| c.id != moving.id) {
            for axis in [GuideAxis::Horizontal, GuideAxis::Vertical] {
                let first = guides.len();
                let (start, end) = span(axis, moving.rect, candidate.rect);
                let counted = GuideReference::along(axis)
                    .into_iter()
                    .filter(|reference| references.is_none_or(|only| only.contains(reference)));
                for dragged_reference in counted {
                    for candidate_reference in GuideReference::along(axis) {
                        let coordinate = candidate_reference.of(candidate.rect);
                        if (dragged_reference.of(moving.rect) - coordinate).abs() > tolerance {
                            continue;
                        }
                        guides.push(AlignmentGuide {
                            axis,
                            coordinate,
                            start,
                            end,
                            dragged: moving.id.clone(),
                            candidate: candidate.id.clone(),
                            dragged_reference,
                            candidate_reference,
                        });
                    }
                }
                drop_implied_centre(&mut guides, first, axis);
            }
        }
    }
    guides
}

/// The extent a guide between two rects runs over along `axis`.
fn span(axis: GuideAxis, a: Rect, b: Rect) -> (f64, f64) {
    match axis {
        GuideAxis::Horizontal => (a.x.min(b.x), (a.x + a.width).max(b.x + b.width)),
        GuideAxis::Vertical => (a.y.min(b.y), (a.y + a.height).max(b.y + b.height)),
    }
}

/// Removes the centre guides of one pair on one axis, `guides[first..]`,
/// when both of the pair's edges are aligned too.
fn drop_implied_centre(guides: &mut Vec<AlignmentGuide>, first: usize, axis: GuideAxis) {
    let [low, high, centre] = GuideReference::along(axis);
    let has = |reference| {
        guides[first..]
            .iter()
            .any(|guide| guide.dragged_reference == reference)
    };
    if has(low) && has(high) {
        let kept: Vec<AlignmentGuide> = (guides.drain(first..))
            .filter(|guide| guide.dragged_reference != centre)
            .collect();
        guides.extend(kept);
    }
}

#[cfg(test)]
mod tests {
    use specular_doc::EntityId;

    use super::*;
    use GuideAxis::{Horizontal, Vertical};
    use GuideReference::{Bottom, HCenter, Left, Right, Top};

    fn rect(id: &str, (x, y, width, height): (f64, f64, f64, f64)) -> SnapCandidate {
        SnapCandidate {
            id: EntityId::new(id),
            rect: Rect::new(x, y, width, height),
        }
    }

    /// Electron's unit cases, `tests/unit/alignment-guide-detector.test.ts`.
    #[test]
    fn aligned_references_are_found_and_an_implied_centre_is_dropped() {
        type Pair = (GuideReference, GuideReference);
        type Box = (f64, f64, f64, f64);
        /// Name, dragged, neighbour, the references counted, axis, found.
        type Row = (
            &'static str,
            Box,
            Box,
            Option<&'static [GuideReference]>,
            GuideAxis,
            &'static [Pair],
        );
        #[rustfmt::skip]
        let rows: [Row; 9] = [
            ("top and bottom, centre implied", (200.0, 100.0, 80.0, 40.0), (20.0, 100.0, 100.0, 40.0), None, Horizontal, &[(Top, Top), (Bottom, Bottom)]),
            ("left and right, centre implied", (100.0, 180.0, 80.0, 40.0), (100.0, 20.0, 80.0, 100.0), None, Vertical, &[(Left, Left), (Right, Right)]),
            ("only the centre", (200.0, 90.0, 80.0, 60.0), (20.0, 100.0, 100.0, 40.0), None, Horizontal, &[(HCenter, HCenter)]),
            ("half a unit off is aligned", (100.5, 10.0, 20.0, 20.0), (100.0, 100.0, 20.0, 20.0), Some(&[Left]), Vertical, &[(Left, Left)]),
            ("a hair more is not", (100.51, 40.0, 20.0, 20.0), (100.0, 100.0, 20.0, 20.0), Some(&[Left]), Vertical, &[]),
            ("nothing aligned across", (13.0, 17.0, 30.0, 30.0), (100.0, 200.0, 40.0, 40.0), None, Horizontal, &[]),
            ("nothing aligned down", (13.0, 17.0, 30.0, 30.0), (100.0, 200.0, 40.0, 40.0), None, Vertical, &[]),
            ("a resize counts its handle's edge", (100.0, 100.0, 80.0, 40.0), (20.0, 40.0, 160.0, 80.0), Some(&[Right]), Vertical, &[(Right, Right)]),
            ("and no other", (100.0, 100.0, 80.0, 40.0), (20.0, 40.0, 160.0, 80.0), Some(&[Right]), Horizontal, &[]),
        ];
        for (name, dragged, candidate, references, axis, want) in rows {
            let found: Vec<Pair> = alignment_guides(
                &[rect("dragged", dragged)],
                &[rect("candidate", candidate)],
                references,
                0.5,
            )
            .into_iter()
            .filter(|guide| guide.axis == axis)
            .map(|guide| (guide.dragged_reference, guide.candidate_reference))
            .collect();
            assert_eq!(found, want, "{name}");
        }
    }

    #[test]
    fn a_guide_spans_both_rects_at_the_neighbours_coordinate() {
        let guides = alignment_guides(
            &[rect("dragged", (200.0, 100.0, 80.0, 40.0))],
            &[rect("candidate", (20.0, 100.0, 100.0, 80.0))],
            None,
            0.5,
        );
        let line = &guides[0];
        assert_eq!(
            (line.axis, line.coordinate, line.start, line.end),
            (Horizontal, 100.0, 20.0, 280.0)
        );
    }
}
