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
}

export interface DescriptorQuery {
  name?: string | null
  text?: string | null
  elementPath?: string | null
  fullPath?: string | null
  interactiveOnly?: boolean
}

export function normalizeSearchText(value: string | null | undefined): string | null {
  if (!value) return null
  const normalized = value.trim().toLowerCase()
  return normalized.length > 0 ? normalized : null
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

  const normalizedName = normalizeSearchText(candidate.name)
  const normalizedText = normalizeSearchText(candidate.text)
  const normalizedElementPath = normalizeSearchText(candidate.elementPath)
  const normalizedFullPath = normalizeSearchText(candidate.fullPath)
  const wantedName = normalizeSearchText(query.name)
  const wantedText = normalizeSearchText(query.text)
  const wantedElementPath = normalizeSearchText(query.elementPath)
  const wantedFullPath = normalizeSearchText(query.fullPath)

  let score = candidate.interactive ? 50 : 0
  let matched = false

  if (wantedName) {
    if (normalizedName === wantedName) {
      score += 400
      matched = true
    } else if (normalizedName?.includes(wantedName)) {
      score += 280
      matched = true
    } else if (normalizedText === wantedName) {
      score += 220
      matched = true
    } else if (normalizedText?.includes(wantedName)) {
      score += 140
      matched = true
    } else {
      return Number.NEGATIVE_INFINITY
    }
  }

  if (wantedText) {
    if (normalizedText === wantedText) {
      score += 320
      matched = true
    } else if (normalizedText?.includes(wantedText)) {
      score += 200
      matched = true
    } else if (normalizedName === wantedText) {
      score += 180
      matched = true
    } else if (normalizedName?.includes(wantedText)) {
      score += 120
      matched = true
    } else {
      return Number.NEGATIVE_INFINITY
    }
  }

  if (wantedElementPath) {
    if (normalizedElementPath === wantedElementPath) {
      score += 260
      matched = true
    } else if (normalizedElementPath?.includes(wantedElementPath)) {
      score += 140
      matched = true
    } else {
      return Number.NEGATIVE_INFINITY
    }
  }

  if (wantedFullPath) {
    if (normalizedFullPath === wantedFullPath) {
      score += 260
      matched = true
    } else if (normalizedFullPath?.includes(wantedFullPath)) {
      score += 140
      matched = true
    } else {
      return Number.NEGATIVE_INFINITY
    }
  }

  if (!matched && (wantedName || wantedText || wantedElementPath || wantedFullPath)) {
    return Number.NEGATIVE_INFINITY
  }

  score += Math.max(0, 100 - candidate.boundsX * 0.01 - candidate.boundsY * 0.01)
  return score
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
