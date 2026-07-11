/**
 * Ambient presence-cursor drift — issue #319 Phase 3.
 *
 * ADR 0029 anchors every *act* to real dispatch time, but says nothing about
 * the unknowable gap between CLI commands (LLM thinking time): a real
 * collaborator's cursor is never perfectly frozen there. This module is the
 * pure math for that ambient motion — which mode applies, and the bounded
 * offset to composite on top of the server-driven spline position.
 *
 * Renderer-only by construction: nothing here touches `lastMoveAt` or
 * writes a position back to main (ADR 0029 rule 4 — no speculative
 * pre-positioning, and a main-side write would corrupt the dwell budget
 * accounting `waitForPresenceDwell` reads). `selectAmbientMode` is also
 * called from main (`canvas-layout-data.ts`) to compute the broadcast's
 * `ambientMode` field, so the classification lives in one place.
 */

import type { PresenceActivity, PresenceLabelKey } from './types'
import type { Vec2 } from './cursor-motion'

export type AmbientDriftMode = 'none' | 'idle-drift' | 'reading-scan'

/** Label keys that represent the agent reading/observing the page rather
 *  than a generic thinking pause. Kept narrow and explicit per the spec
 *  ("inspect_page, snapshot/read labels") rather than "everything that
 *  isn't a mutating act", so new label keys default to the plainer
 *  idle-drift instead of silently inheriting reading-scan's larger radius. */
const OBSERVATION_LABEL_KEYS: ReadonlySet<PresenceLabelKey> = new Set([
  'inspect_page',
  'read_content',
])

/**
 * Which ambient drift mode, if any, applies right now.
 *
 * `activity` gates it: only `waiting`/`thinking` are inter-command-gap
 * states — `traveling`/`acting` mean a real act is in flight and must not
 * fight the spline, `idle`/`departing` mean the session is winding down and
 * ambient motion would read as a ghost still working. `lastIntentLabelKey`
 * is the label key of the agent's most recent *real* intent, preserved
 * across the transition to the synthetic `thinking`/`idle` label (see
 * `lastIntentLabelKey` on `PresenceCursorEntry` in presence-cursor.ts) —
 * it's what lets a reading gap read differently from a generic thinking
 * gap even though both show a "Thinking…" label.
 */
export function selectAmbientMode(
  activity: PresenceActivity,
  lastIntentLabelKey: PresenceLabelKey | null,
): AmbientDriftMode {
  if (activity !== 'waiting' && activity !== 'thinking') return 'none'
  if (lastIntentLabelKey && OBSERVATION_LABEL_KEYS.has(lastIntentLabelKey)) {
    return 'reading-scan'
  }
  return 'idle-drift'
}

interface AmbientDriftParams {
  /** Hard bound on offset magnitude, px. ADR 0029: drift stays near the
   *  truthful position, never speculative or target-seeking. */
  radiusPx: number
  /** Wander period, ms — roughly how long one lazy loop around the rest
   *  position takes. */
  periodMs: number
}

// "hand-at-rest, not screensaver": a few px, gentle.
const IDLE_DRIFT: AmbientDriftParams = { radiusPx: 8, periodMs: 4600 }
// Reading suggests more travel across the content than idling in place,
// but stays clearly ambient — nowhere near the ~30-400px a real travel hop
// covers.
const READING_SCAN: AmbientDriftParams = { radiusPx: 18, periodMs: 3100 }

/** Ease the offset in from zero instead of popping to full amplitude the
 *  instant a mode activates (e.g. the frame `thinking` starts). */
const RAMP_MS = 450

// Golden-ratio frequency ratio between the two summed sine waves per axis
// keeps the wander from ever completing a visible short loop while staying
// perfectly deterministic and cheap (no accumulated per-frame state).
const FREQ_RATIO = 1.618

function driftAxis(seed: number, axisSalt: number, tSec: number, periodSec: number): number {
  const phase = (((seed ^ axisSalt) % 1000) / 1000) * Math.PI * 2
  const f1 = 1 / periodSec
  const f2 = f1 * FREQ_RATIO
  const a = Math.sin(2 * Math.PI * f1 * tSec + phase)
  const b = Math.sin(2 * Math.PI * f2 * tSec + phase * 1.3 + 1.7)
  return a * 0.7 + b * 0.3
}

function clampToRadius(v: Vec2, radius: number): Vec2 {
  const len = Math.hypot(v.x, v.y)
  if (len <= radius || len === 0) return v
  const k = radius / len
  return { x: v.x * k, y: v.y * k }
}

/**
 * Bounded, non-repeating-looking offset to composite on top of a presence
 * cursor's truthful (server-driven) position.
 *
 * Pure function of `(seed, elapsedSinceModeStartMs, mode)` — no RNG, no
 * accumulated state, so callers can resample it every RAF tick without a
 * per-cursor timer. `elapsedSinceModeStartMs` is time since this cursor's
 * ambient mode most recently switched on (not wall-clock time), so the
 * offset always starts at (and near) zero at that instant — see
 * `clampToRadius` and the ramp-in below — rather than popping to an
 * arbitrary point on the wander curve.
 *
 * `mode: 'none'` (real motion in flight, or the session isn't in a gap
 * state) always returns the zero vector: the one invariant every caller
 * must be able to rely on to yield instantly to a real position update.
 */
export function ambientDriftOffset(
  seed: number,
  elapsedSinceModeStartMs: number,
  mode: AmbientDriftMode,
): Vec2 {
  if (mode === 'none' || elapsedSinceModeStartMs <= 0) return { x: 0, y: 0 }
  const params = mode === 'reading-scan' ? READING_SCAN : IDLE_DRIFT
  const tSec = elapsedSinceModeStartMs / 1000
  const periodSec = params.periodMs / 1000
  const nx = driftAxis(seed, 17, tSec, periodSec)
  const ny = driftAxis(seed, 31, tSec, periodSec * 1.15)
  const ramp = Math.min(1, elapsedSinceModeStartMs / RAMP_MS)
  const raw: Vec2 = {
    x: nx * params.radiusPx * ramp,
    y: ny * params.radiusPx * ramp,
  }
  return clampToRadius(raw, params.radiusPx)
}

/** Deterministic per-session seed for `ambientDriftOffset`, derived from
 *  stable cursor identity (sessionId) rather than `Math.random()` — two
 *  cursors don't wander in lockstep, and a given session's drift is
 *  reproducible across renders (matters for the debug playground and for
 *  reasoning about a specific session's replay). */
export function sessionAmbientSeed(sessionId: string): number {
  let hash = 2166136261
  for (let i = 0; i < sessionId.length; i++) {
    hash ^= sessionId.charCodeAt(i)
    hash = Math.imul(hash, 16777619)
  }
  return hash >>> 0
}
