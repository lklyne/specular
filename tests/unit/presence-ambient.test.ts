import { describe, expect, it } from 'vitest'
import {
  ambientDriftOffset,
  selectAmbientMode,
  sessionAmbientSeed,
} from '../../src/shared/presence-ambient'
import type { PresenceActivity, PresenceLabelKey } from '../../src/shared/types'

// Issue #319 Phase 3: ambient cursor drift in the inter-command gap.
// `selectAmbientMode` decides *whether* a cursor drifts (and which flavor);
// `ambientDriftOffset` is the bounded, deterministic offset composited on
// top of the truthful spline position in AgentCursorLayer.tsx. Neither
// function touches DOM or main-process state — that's the point (ADR 0029
// rule 4: no speculative pre-positioning, and a main-side position write
// would reset `lastMoveAt` and corrupt the dwell budget accounting).

describe('selectAmbientMode', () => {
  const cases: Array<{
    label: string
    activity: PresenceActivity
    lastIntentLabelKey: PresenceLabelKey | null
    expected: ReturnType<typeof selectAmbientMode>
  }> = [
    { label: 'traveling never drifts', activity: 'traveling', lastIntentLabelKey: 'inspect_page', expected: 'none' },
    { label: 'acting never drifts', activity: 'acting', lastIntentLabelKey: 'read_content', expected: 'none' },
    { label: 'departing never drifts', activity: 'departing', lastIntentLabelKey: null, expected: 'none' },
    { label: 'idle never drifts (spec covers waiting/thinking only)', activity: 'idle', lastIntentLabelKey: 'inspect_page', expected: 'none' },
    { label: 'thinking with no prior intent -> idle-drift', activity: 'thinking', lastIntentLabelKey: null, expected: 'idle-drift' },
    { label: 'thinking after a mutating intent -> idle-drift', activity: 'thinking', lastIntentLabelKey: 'click_target', expected: 'idle-drift' },
    { label: 'thinking after inspect_page -> reading-scan', activity: 'thinking', lastIntentLabelKey: 'inspect_page', expected: 'reading-scan' },
    { label: 'thinking after read_content -> reading-scan', activity: 'thinking', lastIntentLabelKey: 'read_content', expected: 'reading-scan' },
    { label: 'waiting after inspect_page -> reading-scan', activity: 'waiting', lastIntentLabelKey: 'inspect_page', expected: 'reading-scan' },
    { label: 'waiting with no prior intent -> idle-drift', activity: 'waiting', lastIntentLabelKey: null, expected: 'idle-drift' },
  ]

  for (const { label, activity, lastIntentLabelKey, expected } of cases) {
    it(`${label} -> ${expected}`, () => {
      expect(selectAmbientMode(activity, lastIntentLabelKey)).toBe(expected)
    })
  }
})

describe('ambientDriftOffset', () => {
  it('is the zero vector when mode is none, regardless of elapsed time', () => {
    for (const elapsed of [0, 1, 500, 10_000]) {
      expect(ambientDriftOffset(1234, elapsed, 'none')).toEqual({ x: 0, y: 0 })
    }
  })

  it('is the zero vector at or before the mode just switched on', () => {
    expect(ambientDriftOffset(1234, 0, 'idle-drift')).toEqual({ x: 0, y: 0 })
    expect(ambientDriftOffset(1234, -50, 'reading-scan')).toEqual({ x: 0, y: 0 })
  })

  // The one invariant every caller depends on (ADR 0029: drift stays near
  // the truthful position, never speculative or target-seeking): the offset
  // never exceeds its mode's radius, for any seed and any elapsed time.
  // idle-drift's radius is bounded well under the ≤12px the spec sets;
  // reading-scan is allowed to roam further but must stay clearly ambient
  // (nowhere near a real travel hop's tens-to-hundreds of px).
  it('never exceeds the bounded radius for idle-drift or reading-scan, across seeds and time', () => {
    const seeds = [0, 1, 42, 1234567, sessionAmbientSeed('session-a'), sessionAmbientSeed('session-b')]
    const elapsedSamples = Array.from({ length: 40 }, (_, i) => i * 733) // ~0..28.5s, irregular stride
    const radiusByMode = { 'idle-drift': 12, 'reading-scan': 30 } as const

    for (const mode of ['idle-drift', 'reading-scan'] as const) {
      for (const seed of seeds) {
        for (const elapsed of elapsedSamples) {
          const offset = ambientDriftOffset(seed, elapsed, mode)
          const magnitude = Math.hypot(offset.x, offset.y)
          expect(magnitude).toBeLessThanOrEqual(radiusByMode[mode])
        }
      }
    }
  })

  it('is deterministic: same (seed, elapsed, mode) always produces the same offset', () => {
    const a = ambientDriftOffset(99, 2200, 'reading-scan')
    const b = ambientDriftOffset(99, 2200, 'reading-scan')
    expect(a).toEqual(b)
  })

  it('different seeds produce different wander so cursors do not move in lockstep', () => {
    const a = ambientDriftOffset(sessionAmbientSeed('session-a'), 1800, 'idle-drift')
    const b = ambientDriftOffset(sessionAmbientSeed('session-b'), 1800, 'idle-drift')
    expect(a).not.toEqual(b)
  })
})

describe('sessionAmbientSeed', () => {
  it('is deterministic for a given sessionId', () => {
    expect(sessionAmbientSeed('abc-123')).toBe(sessionAmbientSeed('abc-123'))
  })

  it('is not derived from Math.random (stable across calls in the same process)', () => {
    const seeds = Array.from({ length: 5 }, () => sessionAmbientSeed('stable-session'))
    expect(new Set(seeds).size).toBe(1)
  })
})
