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

#[cfg(test)]
mod tests {
    use specular_doc::Rect;

    use super::*;
    use GuideAxis::{Horizontal, Vertical};

    fn names(ids: &[&str]) -> Vec<String> {
        ids.iter().map(ToString::to_string).collect()
    }

    fn square(id: &str, x: f64, y: f64) -> SnapCandidate {
        SnapCandidate {
            id: EntityId::new(id),
            rect: Rect::new(x, y, 20.0, 20.0),
        }
    }

    /// Electron's unit cases, `tests/unit/distribution-guide-detector.test.ts`.
    #[test]
    fn equal_gaps_around_the_dragged_rect_make_one_chain() {
        type Chain = (f64, Vec<String>, Vec<(f64, f64, f64)>);
        /// Name, where the dragged square is, neighbours, axis, found.
        type Row = (
            &'static str,
            (f64, f64),
            Vec<SnapCandidate>,
            GuideAxis,
            Vec<Chain>,
        );
        #[rustfmt::skip]
        let rows: [Row; 6] = [
            ("a triple", (40.0, 20.0), vec![square("left", 0.0, 20.0), square("right", 80.0, 20.0)], Horizontal,
                vec![(20.0, names(&["left", "right"]), vec![(20.0, 40.0, 30.0), (60.0, 80.0, 30.0)])]),
            ("half a unit off is equal", (40.0, 20.0), vec![square("left", 0.0, 20.0), square("inside", 80.5, 20.0)], Horizontal,
                vec![(20.0, names(&["left", "inside"]), vec![(20.0, 40.0, 30.0), (60.0, 80.5, 30.0)])]),
            ("a hair more is not", (40.0, 20.0), vec![square("left", 0.0, 20.0), square("outside", 80.51, 20.0)], Horizontal, vec![]),
            ("four make one chain, in order", (40.0, 20.0), vec![square("third", 80.0, 20.0), square("fourth", 120.0, 20.0), square("first", 0.0, 20.0)], Horizontal,
                vec![(20.0, names(&["first", "third", "fourth"]), vec![(20.0, 40.0, 30.0), (60.0, 80.0, 30.0), (100.0, 120.0, 30.0)])]),
            ("a column", (20.0, 40.0), vec![square("top", 20.0, 0.0), square("bottom", 20.0, 80.0)], Vertical,
                vec![(20.0, names(&["top", "bottom"]), vec![(20.0, 40.0, 30.0), (60.0, 80.0, 30.0)])]),
            ("a column is no row", (20.0, 40.0), vec![square("top", 20.0, 0.0), square("bottom", 20.0, 80.0)], Horizontal, vec![]),
        ];
        for (name, (x, y), candidates, axis, want) in rows {
            let found: Vec<Chain> =
                distribution_guides(&square("dragged", x, y), &candidates, axis, 0.5)
                    .into_iter()
                    .map(|guide| {
                        assert_eq!(guide.axis, axis, "{name}");
                        (
                            guide.gap,
                            guide.candidates.iter().map(ToString::to_string).collect(),
                            (guide.gaps.iter())
                                .map(|gap| (gap.start, gap.end, gap.cross))
                                .collect(),
                        )
                    })
                    .collect();
            assert_eq!(found, want, "{name}");
        }
    }
}
