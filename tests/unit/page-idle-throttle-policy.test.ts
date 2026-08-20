// Protects the rules that decide when page renderers may be CPU-throttled.
// The regression that matters is a page going quiet under an agent: agents
// drive this app while it sits unfocused, so activity and awake holds must
// outrank "the window is not focused".
//
// Mutation-verified by replacing the `Math.max(blurredAt + graceMs,
// agentActiveUntil)` in evaluateIdleThrottle with `blurredAt + graceMs` and
// confirming the agent-activity cases below fail.

import { describe, expect, it } from 'vitest'
import { evaluateIdleThrottle } from '../../src/main/runtime/page-idle-policy'

const base = {
  now: 10_000,
  windowFocused: false,
  awakeHoldCount: 0,
  blurredAt: 0,
  agentActiveUntil: 0,
  graceMs: 5_000,
}

describe('idle throttle policy', () => {
  it('never idles a focused window', () => {
    expect(evaluateIdleThrottle({ ...base, windowFocused: true })).toEqual({
      idle: false,
      nextCheckAt: null,
    })
  })

  it('idles once the blur grace has elapsed', () => {
    expect(evaluateIdleThrottle(base).idle).toBe(true)
  })

  it('holds off during the blur grace and reports when to look again', () => {
    expect(evaluateIdleThrottle({ ...base, now: 3_000 })).toEqual({
      idle: false,
      nextCheckAt: 5_000,
    })
  })

  it('stays awake while agent activity is in its trailing window', () => {
    expect(evaluateIdleThrottle({ ...base, agentActiveUntil: 12_000 })).toEqual({
      idle: false,
      nextCheckAt: 12_000,
    })
  })

  it('idles once agent activity lapses, even long after blur', () => {
    expect(evaluateIdleThrottle({ ...base, now: 99_000, agentActiveUntil: 12_000 }).idle).toBe(true)
  })

  it('stays awake while a hold is outstanding, with no timer to expire it', () => {
    expect(evaluateIdleThrottle({ ...base, awakeHoldCount: 1 })).toEqual({
      idle: false,
      nextCheckAt: null,
    })
  })

  it('idles only after every hold is released', () => {
    expect(evaluateIdleThrottle({ ...base, awakeHoldCount: 2 }).idle).toBe(false)
    expect(evaluateIdleThrottle({ ...base, awakeHoldCount: 0 }).idle).toBe(true)
  })
})
