//! The scoring behind interaction sync (ADR 0030).
//!
//! A source page captures each hover or click as a [`LocatorBundle`], a
//! semantic description of the element and not a coordinate. Every peer scores
//! that bundle against its own live elements ([`LocatorCandidate`]) to decide
//! whether and where to replay. The rule is semantic, not positional, and
//! confident-or-skip: any dispatched input must resolve to a confident
//! element match or be refused, because a silent wrong click is worse than no
//! sync.
//!
//! A match is confident when either a unique identity key (id, then test id)
//! singles out one candidate, or the top structural score clears both
//! [`LOCATOR_CONFIDENCE_FLOOR`] and the runner-up by
//! [`LOCATOR_RUNNER_UP_MARGIN`]. Pure: no I/O, no page access.

use serde::{Deserialize, Deserializer, Serialize};

/// Minimum top score for a structural (non-identity) confident match.
pub const LOCATOR_CONFIDENCE_FLOOR: f64 = 300.0;

/// How far the top structural score must clear the runner-up to be confident.
pub const LOCATOR_RUNNER_UP_MARGIN: f64 = 120.0;

/// A rectangle in the page's own content coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocatorRect {
    /// Left edge.
    pub x: f64,
    /// Top edge.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// A semantic capture of the element under the pointer: the wire format shared
/// by the synced cursor, click replay and text-input sync.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocatorBundle {
    /// The element's `id` attribute.
    pub id: Option<String>,
    /// The element's test id (`data-testid`).
    pub test_id: Option<String>,
    /// The ARIA role.
    pub role: Option<String>,
    /// The accessible name.
    pub name: Option<String>,
    /// The visible text.
    pub text: Option<String>,
    /// The tag name.
    #[serde(deserialize_with = "null_as_default")]
    pub tag: String,
    /// The path from the nearest landmark to the element.
    #[serde(deserialize_with = "null_as_default")]
    pub element_path: String,
    /// The path from the document root to the element.
    #[serde(deserialize_with = "null_as_default")]
    pub full_path: String,
    /// Within-element pointer offset as a fraction 0..1 of the width.
    pub offset_x: f64,
    /// Within-element pointer offset as a fraction 0..1 of the height.
    pub offset_y: f64,
}

/// One live element a peer offers as a possible match, enumerated from its own
/// tree (including open shadow roots).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocatorCandidate {
    /// The element's `id` attribute.
    pub id: Option<String>,
    /// The element's test id.
    pub test_id: Option<String>,
    /// The ARIA role.
    pub role: Option<String>,
    /// The accessible name.
    pub name: Option<String>,
    /// The visible text.
    pub text: Option<String>,
    /// The tag name.
    pub tag: Option<String>,
    /// The path from the nearest landmark to the element.
    pub element_path: Option<String>,
    /// The path from the document root to the element.
    pub full_path: Option<String>,
    /// Whether the element takes input (a control or a link).
    pub interactive: bool,
    /// The element's rect in page content coordinates.
    pub rect: LocatorRect,
}

/// The outcome of [`resolve_locator`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocatorResolution {
    /// One candidate is a confident match; dispatch input at `point`.
    Confident {
        /// Index into the candidates slice.
        candidate: usize,
        /// Where to dispatch, inside the candidate's rect.
        point: (f64, f64),
    },
    /// Several candidates fit equally well; refuse rather than guess.
    Ambiguous,
    /// Nothing fits well enough.
    None,
}

fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// The point to dispatch input at inside a resolved candidate's rect, given the
/// source's within-element offset fraction. The offset is clamped to 0..1 so a
/// stale or out-of-range fraction still lands on the element rather than
/// beside it.
pub fn dispatch_point(rect: LocatorRect, offset_x: f64, offset_y: f64) -> (f64, f64) {
    let fx = offset_x.clamp(0.0, 1.0);
    let fy = offset_y.clamp(0.0, 1.0);
    (
        (rect.x + fx * rect.width).clamp(rect.x, rect.x + rect.width),
        (rect.y + fy * rect.height).clamp(rect.y, rect.y + rect.height),
    )
}

fn normalize(value: Option<&str>) -> Option<String> {
    let normalized = value?.trim().to_lowercase();
    (!normalized.is_empty()).then_some(normalized)
}

/// One tier a requested field can match: a candidate value, the points for an
/// exact match and the points for a substring match. Tiers are tried in order
/// and the first hit wins, so they are listed strongest first.
type Tier<'a> = (Option<&'a str>, f64, f64);

/// Points for one requested field, `NEG_INFINITY` if no tier matched (a hard
/// reject: the field was asked for and the candidate cannot satisfy it), and 0
/// when the field was not requested.
fn score_field(wanted: Option<&str>, tiers: &[Tier<'_>]) -> f64 {
    let Some(wanted) = wanted else { return 0.0 };
    for &(value, exact, partial) in tiers {
        let Some(value) = value else { continue };
        if value == wanted {
            return exact;
        }
        if value.contains(wanted) {
            return partial;
        }
    }
    f64::NEG_INFINITY
}

/// Points for a modest tiebreak signal: agreement scores, disagreement is free.
fn score_agreement(wanted: Option<&str>, value: Option<&str>, points: f64) -> f64 {
    match (wanted, value) {
        (Some(wanted), Some(value)) if wanted == value => points,
        _ => 0.0,
    }
}

/// How well a candidate matches the bundle's structural fields. Higher is
/// better; `NEG_INFINITY` is a hard reject.
fn score(bundle: &LocatorBundle, candidate: &LocatorCandidate) -> f64 {
    let norm = |value: &Option<String>| normalize(value.as_deref());
    let name = norm(&candidate.name);
    let text = norm(&candidate.text);
    let element_path = norm(&candidate.element_path);
    let full_path = norm(&candidate.full_path);
    let role = norm(&candidate.role);
    let tag = norm(&candidate.tag);

    let want_name = norm(&bundle.name);
    let want_text = norm(&bundle.text);
    // An empty path or tag is "not requested".
    let want_element_path = normalize(Some(&bundle.element_path));
    let want_full_path = normalize(Some(&bundle.full_path));
    let want_role = norm(&bundle.role);
    let want_tag = normalize(Some(&bundle.tag));

    // A requested name may be satisfied by the candidate's text (and vice
    // versa), at a discount: the accessible name and the visible text are often
    // the same string surfaced two ways.
    let fields = score_field(
        want_name.as_deref(),
        &[
            (name.as_deref(), 400.0, 280.0),
            (text.as_deref(), 220.0, 140.0),
        ],
    ) + score_field(
        want_text.as_deref(),
        &[
            (text.as_deref(), 320.0, 200.0),
            (name.as_deref(), 180.0, 120.0),
        ],
    ) + score_field(
        want_element_path.as_deref(),
        &[(element_path.as_deref(), 260.0, 140.0)],
    ) + score_field(
        want_full_path.as_deref(),
        &[(full_path.as_deref(), 260.0, 140.0)],
    );
    if fields == f64::NEG_INFINITY {
        return f64::NEG_INFINITY;
    }

    // Role and tag agreement separates otherwise-equal structural matches (two
    // same-text controls where one is a button and the other a link). Together
    // they can clear the runner-up margin, promoting a would-be ambiguous match.
    let agreement = score_agreement(want_role.as_deref(), role.as_deref(), 80.0)
        + score_agreement(want_tag.as_deref(), tag.as_deref(), 40.0);

    let proximity = (100.0 - candidate.rect.x * 0.01 - candidate.rect.y * 0.01).max(0.0);
    (if candidate.interactive { 50.0 } else { 0.0 }) + fields + agreement + proximity
}

/// Identity keys in priority order, each reading the key off a bundle and a
/// candidate.
type KeyOf<T> = fn(&T) -> Option<&str>;
const IDENTITY_KEYS: [(KeyOf<LocatorBundle>, KeyOf<LocatorCandidate>); 2] = [
    (|b| b.id.as_deref(), |c| c.id.as_deref()),
    (|b| b.test_id.as_deref(), |c| c.test_id.as_deref()),
];

fn resolve_by_identity_key(
    bundle: &LocatorBundle,
    candidates: &[LocatorCandidate],
) -> Option<LocatorResolution> {
    for (bundle_key, candidate_key) in IDENTITY_KEYS {
        let Some(wanted) = bundle_key(bundle).filter(|wanted| !wanted.is_empty()) else {
            continue;
        };
        let mut matches = candidates
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate_key(candidate) == Some(wanted));
        let (first, second) = (matches.next(), matches.next());
        match (first, second) {
            (Some((index, candidate)), None) => {
                return Some(confident(bundle, index, candidate));
            }
            // A duplicated identity key is genuinely ambiguous: refuse rather
            // than pick the first. Zero matches falls through to the next key
            // and then to structural scoring.
            (Some(_), Some(_)) => return Some(LocatorResolution::Ambiguous),
            _ => {}
        }
    }
    None
}

fn confident(
    bundle: &LocatorBundle,
    index: usize,
    candidate: &LocatorCandidate,
) -> LocatorResolution {
    LocatorResolution::Confident {
        candidate: index,
        point: dispatch_point(candidate.rect, bundle.offset_x, bundle.offset_y),
    }
}

/// Resolve a captured bundle against a peer's live candidates. Identity keys
/// win outright; otherwise the top structural score must clear the floor and
/// the runner-up margin. A confident result carries the dispatch point so
/// callers never recompute the offset geometry.
pub fn resolve_locator(
    bundle: &LocatorBundle,
    candidates: &[LocatorCandidate],
) -> LocatorResolution {
    if candidates.is_empty() {
        return LocatorResolution::None;
    }
    if let Some(resolution) = resolve_by_identity_key(bundle, candidates) {
        return resolution;
    }

    let mut best: Option<usize> = None;
    let mut best_score = f64::NEG_INFINITY;
    let mut runner_up = f64::NEG_INFINITY;
    for (index, candidate) in candidates.iter().enumerate() {
        let score = score(bundle, candidate);
        if score > best_score {
            runner_up = best_score;
            best = Some(index);
            best_score = score;
        } else if score > runner_up {
            runner_up = score;
        }
    }

    let Some(best) =
        best.filter(|_| best_score.is_finite() && best_score >= LOCATOR_CONFIDENCE_FLOOR)
    else {
        return LocatorResolution::None;
    };
    if runner_up.is_finite() && best_score - runner_up < LOCATOR_RUNNER_UP_MARGIN {
        return LocatorResolution::Ambiguous;
    }
    confident(bundle, best, &candidates[best])
}
