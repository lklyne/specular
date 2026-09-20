import { describe, it, expect } from 'vitest'
import { buildChainedPresenceSteps } from '../../src/main/shared/browse-handler'

// A chained browse command (`click @e1 && fill @e2 "x" && click @e3`) runs as
// a single `batch` spawn — there's no HTTP round trip between steps to fire
// a fresh presence intent from. `buildChainedPresenceSteps` is the pure
// extraction that turns the chain into the ordered list main queues and
// advances through (`/session/presence/intent`'s `queue` field,
// `advancePendingIntent` in routes/session.ts).
describe('buildChainedPresenceSteps', () => {
  it('builds one step per mappable command, in order', () => {
    expect(buildChainedPresenceSteps(['click @e1', 'fill @e2 "hello"', 'click @e3'])).toEqual([
      {
        labelKey: 'click_target',
        command: 'click',
        targetRef: '@e1',
        targetRefSource: 'agent-browser',
        targetQuery: null,
        labelHint: null,
      },
      {
        labelKey: 'type_text',
        command: 'fill',
        targetRef: '@e2',
        targetRefSource: 'agent-browser',
        targetQuery: null,
        labelHint: 'editing control',
      },
      {
        labelKey: 'click_target',
        command: 'click',
        targetRef: '@e3',
        targetRefSource: 'agent-browser',
        targetQuery: null,
        labelHint: null,
      },
    ])
  })

  it('carries a re-resolving target query for a selector/text step, not just a ref', () => {
    expect(buildChainedPresenceSteps(['click "#submit"'])).toEqual([
      {
        labelKey: 'click_target',
        command: 'click',
        targetRef: null,
        targetRefSource: null,
        targetQuery: { selector: '#submit', text: null, name: null },
        labelHint: null,
      },
    ])
  })

  it('drops a step with no mappable label instead of leaving a hole in the queue', () => {
    // `press` has no COMMAND_LABELS entry — buildChainedPresenceSteps must
    // skip it rather than emit a null/undefined placeholder that would
    // desync the queue against the actual advance signals.
    const steps = buildChainedPresenceSteps(['click @e1', 'press Enter', 'click @e2'])
    expect(steps.map((s) => s.command)).toEqual(['click', 'click'])
  })

  it('returns an empty list for a chain with no mappable commands', () => {
    expect(buildChainedPresenceSteps(['press Enter'])).toEqual([])
  })
})
