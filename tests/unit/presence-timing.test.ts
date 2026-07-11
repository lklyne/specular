import { describe, it, expect } from 'vitest'
import {
  PRESENCE_BURST_STEP_DELAY_MS,
  PRESENCE_BURST_WINDOW_MS,
  PRESENCE_STEP_DELAY_MS,
  computeDwellRemainingMs,
  selectDwellBudgetMs,
} from '../../src/shared/presence-timing'

// Issue #319 Phase 2: the pre-act dwell (ADR 0029) pays a short budget when
// the session is mid-burst (continuous cursor motion already visible) and
// the full budget after a gap (agent was thinking, so the dwell is
// perceptually free). selectDwellBudgetMs is the pure regime-selection
// helper the CDP proxy calls before each mutating dispatch.
describe('selectDwellBudgetMs', () => {
  const cases: Array<{ label: string; msSinceLastAct: number | null; expected: number }> = [
    { label: 'no prior act (first act of a session)', msSinceLastAct: null, expected: PRESENCE_STEP_DELAY_MS },
    { label: 'act landed 0ms ago', msSinceLastAct: 0, expected: PRESENCE_BURST_STEP_DELAY_MS },
    { label: 'act landed well inside the burst window', msSinceLastAct: 200, expected: PRESENCE_BURST_STEP_DELAY_MS },
    {
      label: 'act landed just under the burst window',
      msSinceLastAct: PRESENCE_BURST_WINDOW_MS - 1,
      expected: PRESENCE_BURST_STEP_DELAY_MS,
    },
    {
      label: 'act landed exactly at the burst window boundary',
      msSinceLastAct: PRESENCE_BURST_WINDOW_MS,
      expected: PRESENCE_STEP_DELAY_MS,
    },
    {
      label: 'act landed just past the burst window',
      msSinceLastAct: PRESENCE_BURST_WINDOW_MS + 1,
      expected: PRESENCE_STEP_DELAY_MS,
    },
    { label: 'act landed long ago (thinking gap)', msSinceLastAct: 5_000, expected: PRESENCE_STEP_DELAY_MS },
  ]

  for (const { label, msSinceLastAct, expected } of cases) {
    it(`${label} -> ${expected}ms`, () => {
      expect(selectDwellBudgetMs(msSinceLastAct)).toBe(expected)
    })
  }
})

// Issue #319 Phase 5: computeDwellRemainingMs is the max(0, budget - elapsed)
// math `waitForPresenceDwell` (app-control-server.ts) used to inline. Kept
// event-shaped (plain timestamps) so it ports unchanged to the future
// presence event timeline (ADR 0029).
describe('computeDwellRemainingMs', () => {
  const cases: Array<{
    label: string
    lastMoveAt: number
    budgetMs: number
    now: number
    expected: number
  }> = [
    { label: 'no time elapsed', lastMoveAt: 1_000, budgetMs: 300, now: 1_000, expected: 300 },
    { label: 'partway through the budget', lastMoveAt: 1_000, budgetMs: 300, now: 1_100, expected: 200 },
    { label: 'exactly-elapsed budget', lastMoveAt: 1_000, budgetMs: 300, now: 1_300, expected: 0 },
    { label: 'elapsed past the budget', lastMoveAt: 1_000, budgetMs: 300, now: 1_500, expected: 0 },
    { label: 'zero budget', lastMoveAt: 1_000, budgetMs: 0, now: 1_000, expected: 0 },
  ]

  for (const { label, lastMoveAt, budgetMs, now, expected } of cases) {
    it(`${label} -> ${expected}ms`, () => {
      expect(computeDwellRemainingMs(lastMoveAt, budgetMs, now)).toBe(expected)
    })
  }
})
