import { describe, it, expect } from 'vitest'
import {
  PRESENCE_BURST_STEP_DELAY_MS,
  PRESENCE_BURST_WINDOW_MS,
  PRESENCE_STEP_DELAY_MS,
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
