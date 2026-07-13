// The locator kernel: the shared, pure resolution logic behind interaction
// sync (ADR 0030). A source page captures each hover/click as a `LocatorBundle`
// — a semantic description of the element, not a coordinate — and every peer
// scores that bundle against its own live DOM to decide *whether* and *where*
// to replay. "Semantic, not positional": display may glide proportionally, but
// any dispatched input must resolve to a confident element match or be refused.
//
// Pure by contract (src/shared layer rule): no side effects, no process- or
// DOM-specific imports. Both the guest preload (resolver) and main (scoring
// parity with presence targeting) build against this one implementation.

/** A rectangle in the page's own content coordinate space. */
export interface LocatorRect {
  x: number
  y: number
  width: number
  height: number
}

/**
 * The wire format shared by the synced cursor, click replay, and (later)
 * text-input sync. Changing its shape after text sync lands touches capture,
 * fan-out, and every resolver — keep it deliberate (ADR 0030 consequences).
 */
export interface LocatorBundle {
  id?: string
  testId?: string
  role?: string
  name?: string
  text?: string
  tag: string
  elementPath: string
  fullPath: string
  /** Within-element pointer offset as a fraction 0..1 of width/height. */
  offsetX: number
  offsetY: number
}

/**
 * One live-DOM element a peer offers as a possible match. The resolver
 * enumerates these from its own tree (including open shadow roots) and scores
 * each against the incoming bundle.
 */
export interface LocatorCandidate {
  id: string | null
  testId: string | null
  role: string | null
  name: string | null
  text: string | null
  tag: string | null
  elementPath: string | null
  fullPath: string | null
  interactive: boolean
  rect: LocatorRect
}

export type LocatorResolution =
  | { kind: 'confident'; candidate: LocatorCandidate; point: { x: number; y: number } }
  | { kind: 'ambiguous' }
  | { kind: 'none' }

// --- Confidence policy (ADR 0030, decision D2) ---
//
// A match is confident when EITHER a unique identity key (id, then testId)
// singles out exactly one candidate, OR the top structural score clears both an
// absolute floor and the runner-up by a margin. Anything short of that — most
// importantly two same-text buttons — is refused rather than guessed, because a
// silent wrong-click is worse than no sync.

/** Minimum top score for a structural (non-identity) confident match. */
export const LOCATOR_CONFIDENCE_FLOOR = 300

/** How far the top structural score must clear the runner-up to be confident. */
export const LOCATOR_RUNNER_UP_MARGIN = 120

// --- Descriptor scoring (shared vocabulary with presence targeting) ---

/**
 * The structural fields a descriptor match scores over. Mirrors the candidate
 * shape `findPresenceTarget` scores so main and the peer resolver rank
 * elements by one implementation.
 */
export interface DescriptorScoreInput {
  name: string | null
  text: string | null
  elementPath: string | null
  fullPath: string | null
  interactive: boolean
  boundsX: number
  boundsY: number
  // ARIA role and tag name are weak tiebreak signals: agreement nudges the
  // score, a mismatch never rejects. Optional so presence targeting (which
  // scores agent-snapshot nodes without them) keeps its existing behavior.
  role?: string | null
  tag?: string | null
}

export interface DescriptorQuery {
  name?: string | null
  text?: string | null
  elementPath?: string | null
  fullPath?: string | null
  role?: string | null
  tag?: string | null
  interactiveOnly?: boolean
}

function normalizeSearchText(value: string | null | undefined): string | null {
  if (!value) return null
  const normalized = value.trim().toLowerCase()
  return normalized.length > 0 ? normalized : null
}

/** One tier a requested field can match against: a candidate value, the points
 *  for an exact match, and the points for a substring match. Tiers are tried in
 *  order — the first hit wins, so they are listed strongest-first. */
type MatchTier = [value: string | null, exact: number, partial: number]

/** Points for one requested field, or `NEGATIVE_INFINITY` if no tier matched (a
 *  hard reject — the field was asked for and the candidate cannot satisfy it).
 *  Returns 0 when the field was not requested. */
function scoreField(wanted: string | null, tiers: MatchTier[]): number {
  if (!wanted) return 0
  for (const [value, exact, partial] of tiers) {
    if (value === wanted) return exact
    if (value?.includes(wanted)) return partial
  }
  return Number.NEGATIVE_INFINITY
}

/** Points for a modest tiebreak signal: agreement scores, disagreement is free.
 *  Ranks below the identity tiers, so a mismatch is never a hard reject. */
function scoreAgreement(wanted: string | null | undefined, value: string | null | undefined, points: number): number {
  const wantedNorm = normalizeSearchText(wanted)
  if (!wantedNorm) return 0
  return normalizeSearchText(value) === wantedNorm ? points : 0
}

/**
 * Score how well a candidate matches a descriptor query. Higher is better;
 * `Number.NEGATIVE_INFINITY` means a specified field could not be matched at
 * all (a hard reject, not a weak match). The weights are the presence
 * targeting weights — name/text/path exact-or-includes tiers, an interactive
 * bonus, and a top-left proximity tiebreak — so mirrored input and agent
 * targeting resolve elements identically.
 */
export function scoreDescriptorMatch(
  candidate: DescriptorScoreInput,
  query: DescriptorQuery,
): number {
  if (query.interactiveOnly && !candidate.interactive) return Number.NEGATIVE_INFINITY

  const name = normalizeSearchText(candidate.name)
  const text = normalizeSearchText(candidate.text)
  const elementPath = normalizeSearchText(candidate.elementPath)
  const fullPath = normalizeSearchText(candidate.fullPath)

  // A requested name may be satisfied by the candidate's text (and vice versa),
  // at a discount — the accessible name and the visible text are often the same
  // string surfaced two ways.
  const fields =
    scoreField(normalizeSearchText(query.name), [[name, 400, 280], [text, 220, 140]]) +
    scoreField(normalizeSearchText(query.text), [[text, 320, 200], [name, 180, 120]]) +
    scoreField(normalizeSearchText(query.elementPath), [[elementPath, 260, 140]]) +
    scoreField(normalizeSearchText(query.fullPath), [[fullPath, 260, 140]])
  if (fields === Number.NEGATIVE_INFINITY) return Number.NEGATIVE_INFINITY

  // Role/tag agreement separates otherwise-equal structural matches (two
  // same-text controls where one is a <button> and the other a link). Together
  // they can clear the runner-up margin, promoting a would-be ambiguous match.
  const agreement =
    scoreAgreement(query.role, candidate.role, 80) + scoreAgreement(query.tag, candidate.tag, 40)

  const proximity = Math.max(0, 100 - candidate.boundsX * 0.01 - candidate.boundsY * 0.01)
  return (candidate.interactive ? 50 : 0) + fields + agreement + proximity
}

// --- Dispatch geometry ---

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

/**
 * The point to dispatch input at inside a resolved candidate's rect, given the
 * source's within-element offset fraction. The offset is clamped to 0..1 (and
 * the result to the rect) so a stale or out-of-range fraction still lands on
 * the element rather than beside it.
 */
export function dispatchPointForCandidate(
  rect: LocatorRect,
  offsetX: number,
  offsetY: number,
): { x: number; y: number } {
  const fx = clamp(offsetX, 0, 1)
  const fy = clamp(offsetY, 0, 1)
  return {
    x: clamp(rect.x + fx * rect.width, rect.x, rect.x + rect.width),
    y: clamp(rect.y + fy * rect.height, rect.y, rect.y + rect.height),
  }
}

// --- Resolution ---

const IDENTITY_KEYS = ['id', 'testId'] as const

function resolveByIdentityKey(
  bundle: LocatorBundle,
  candidates: LocatorCandidate[],
): LocatorResolution | null {
  for (const key of IDENTITY_KEYS) {
    const wanted = bundle[key]
    if (!wanted) continue
    const matches = candidates.filter((candidate) => candidate[key] === wanted)
    if (matches.length === 1) {
      return {
        kind: 'confident',
        candidate: matches[0],
        point: dispatchPointForCandidate(matches[0].rect, bundle.offsetX, bundle.offsetY),
      }
    }
    // A duplicated identity key is genuinely ambiguous — refuse rather than
    // pick the first. Zero matches for this key falls through to the next key
    // and then to structural scoring.
    if (matches.length > 1) return { kind: 'ambiguous' }
  }
  return null
}

function toScoreInput(candidate: LocatorCandidate): DescriptorScoreInput {
  return {
    name: candidate.name,
    text: candidate.text,
    elementPath: candidate.elementPath,
    fullPath: candidate.fullPath,
    interactive: candidate.interactive,
    boundsX: candidate.rect.x,
    boundsY: candidate.rect.y,
    role: candidate.role,
    tag: candidate.tag,
  }
}

/**
 * Resolve a captured bundle against a peer's live candidates. Identity keys win
 * outright; otherwise the top structural score must clear the floor and the
 * runner-up margin. Returns the dispatch point for confident matches so callers
 * never recompute the offset geometry.
 */
export function resolveLocator(
  bundle: LocatorBundle,
  candidates: LocatorCandidate[],
): LocatorResolution {
  if (candidates.length === 0) return { kind: 'none' }

  const identity = resolveByIdentityKey(bundle, candidates)
  if (identity) return identity

  const query: DescriptorQuery = {
    name: bundle.name ?? null,
    text: bundle.text ?? null,
    elementPath: bundle.elementPath ?? null,
    fullPath: bundle.fullPath ?? null,
    role: bundle.role ?? null,
    tag: bundle.tag ?? null,
  }

  let best: LocatorCandidate | null = null
  let bestScore = Number.NEGATIVE_INFINITY
  let runnerUpScore = Number.NEGATIVE_INFINITY
  for (const candidate of candidates) {
    const score = scoreDescriptorMatch(toScoreInput(candidate), query)
    if (score > bestScore) {
      runnerUpScore = bestScore
      best = candidate
      bestScore = score
    } else if (score > runnerUpScore) {
      runnerUpScore = score
    }
  }

  if (!best || !Number.isFinite(bestScore) || bestScore < LOCATOR_CONFIDENCE_FLOOR) {
    return { kind: 'none' }
  }
  if (Number.isFinite(runnerUpScore) && bestScore - runnerUpScore < LOCATOR_RUNNER_UP_MARGIN) {
    return { kind: 'ambiguous' }
  }
  return {
    kind: 'confident',
    candidate: best,
    point: dispatchPointForCandidate(best.rect, bundle.offsetX, bundle.offsetY),
  }
}
