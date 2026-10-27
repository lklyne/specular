//! The distribution detector: runs of neighbours the dragged rect sits in
//! at equal spacing.

use specular_doc::EntityId;

use super::{DistributionGap, DistributionGuide, GuideAxis, SnapCandidate};

/// A rect seen along one axis.
struct Item<'a> {
    id: &'a EntityId,
    start: f64,
    end: f64,
    cross_start: f64,
    cross_end: f64,
    dragged: bool,
}

impl<'a> Item<'a> {
    fn new(candidate: &'a SnapCandidate, axis: GuideAxis, dragged: bool) -> Self {
        let rect = candidate.rect;
        let (start, length, cross_start, cross_length) = match axis {
            GuideAxis::Horizontal => (rect.x, rect.width, rect.y, rect.height),
            GuideAxis::Vertical => (rect.y, rect.height, rect.x, rect.width),
        };
        Self {
            id: &candidate.id,
            start,
            end: start + length,
            cross_start,
            cross_end: cross_start + cross_length,
            dragged,
        }
    }
}

/// The space between two neighbours in the sorted run.
struct Between {
    gap: f64,
    /// Whether the two overlap across the axis and do not overlap along it,
    /// so the gap is one a mark can sit in.
    usable: bool,
}

/// The equal-spacing chains along `axis` that include `dragged`: at least
/// two equal gaps in a row, between rects that face each other, with two or
/// more neighbours in the chain. One guide per chain.
pub(crate) fn distribution_guides(
    dragged: &SnapCandidate,
    candidates: &[SnapCandidate],
    axis: GuideAxis,
    tolerance: f64,
) -> Vec<DistributionGuide> {
    let mut items: Vec<Item<'_>> = std::iter::once(Item::new(dragged, axis, true))
        .chain(
            (candidates.iter())
                .filter(|candidate| candidate.id != dragged.id)
                .map(|candidate| Item::new(candidate, axis, false)),
        )
        .collect();
    items.sort_by(|a, b| {
        (a.start.total_cmp(&b.start))
            .then(a.end.total_cmp(&b.end))
            .then_with(|| a.id.as_str().cmp(b.id.as_str()))
    });
    let Some(at) = items.iter().position(|item| item.dragged) else {
        return Vec::new();
    };
    let between: Vec<Between> = (items.windows(2))
        .map(|pair| Between {
            gap: pair[1].start - pair[0].end,
            usable: pair[1].start - pair[0].end >= 0.0
                && pair[0].cross_end.min(pair[1].cross_end)
                    > pair[0].cross_start.max(pair[1].cross_start),
        })
        .collect();
    let joins = |index: usize, gap: f64| {
        between[index].usable && (between[index].gap - gap).abs() <= tolerance
    };

    let mut guides = Vec::new();
    let mut seen: Vec<(usize, usize)> = Vec::new();
    for (anchor, space) in between.iter().enumerate() {
        if !space.usable {
            continue;
        }
        let mut first = anchor;
        while first > 0 && joins(first - 1, space.gap) {
            first -= 1;
        }
        let mut last = anchor;
        while last + 1 < between.len() && joins(last + 1, space.gap) {
            last += 1;
        }
        if last - first + 1 < 2 || at < first || at > last + 1 || seen.contains(&(first, last)) {
            continue;
        }
        let chain = &items[first..=last + 1];
        if chain.iter().filter(|item| !item.dragged).count() < 2 {
            continue;
        }
        seen.push((first, last));
        guides.push(guide(&dragged.id, axis, chain));
    }
    guides
}

fn guide(dragged: &EntityId, axis: GuideAxis, chain: &[Item<'_>]) -> DistributionGuide {
    let gaps: Vec<DistributionGap> = (chain.windows(2))
        .map(|pair| DistributionGap {
            start: pair[0].end,
            end: pair[1].start,
            cross: f64::midpoint(
                pair[0].cross_start.max(pair[1].cross_start),
                pair[0].cross_end.min(pair[1].cross_end),
            ),
        })
        .collect();
    DistributionGuide {
        axis,
        gap: gaps.first().map_or(0.0, |gap| gap.end - gap.start),
        dragged: dragged.clone(),
        candidates: (chain.iter())
            .filter(|item| !item.dragged)
            .map(|item| item.id.clone())
            .collect(),
        gaps,
    }
}
