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
