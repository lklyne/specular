import { describe, it, expect } from 'vitest'
import {
  extractRectFromCdpResult,
  recordPendingRectRequest,
  type CdpProxyRegistration,
} from '../../src/main/cdp-proxy'
import { isMutatingIntentCommand } from '../../src/main/presence-manager'

// Issue #319: agent-browser resolves an @eN ref's coordinates via CDP
// (DOM.getBoxModel / Runtime.callFunctionOn) before dispatching the click.
// extractRectFromCdpResult sniffs those responses for real element geometry
// so the presence cursor can pre-position instead of paying the full
// pre-act dwell on mousePressed. It must stay conservative: read-path calls
// (snapshot, eval) share Runtime.callFunctionOn constantly, and returning a
// rect for an ambiguous value would move the cursor during a plain read.
describe('extractRectFromCdpResult', () => {
  it('parses a DOM.getBoxModel content quad into a bounding rect', () => {
    const result = {
      model: {
        // content quad: top-left, top-right, bottom-right, bottom-left
        content: [10, 20, 110, 20, 110, 70, 10, 70],
      },
    }
    expect(extractRectFromCdpResult('DOM.getBoxModel', result)).toEqual({
      x: 10,
      y: 20,
      width: 100,
      height: 50,
    })
  })

  it('handles a rotated/non-axis-aligned quad by taking the bounding box', () => {
    const result = { model: { content: [0, 10, 10, 0, 20, 10, 10, 20] } }
    expect(extractRectFromCdpResult('DOM.getBoxModel', result)).toEqual({
      x: 0,
      y: 0,
      width: 20,
      height: 20,
    })
  })

  it('returns null when the model is missing', () => {
    expect(extractRectFromCdpResult('DOM.getBoxModel', {})).toBeNull()
  })

  it('returns null when the content quad has the wrong length', () => {
    const result = { model: { content: [0, 0, 10, 10] } }
    expect(extractRectFromCdpResult('DOM.getBoxModel', result)).toBeNull()
  })

  it('returns null when the content quad has non-numeric entries', () => {
    const result = { model: { content: [0, 0, 10, 0, 10, '10', 0, 10] } }
    expect(extractRectFromCdpResult('DOM.getBoxModel', result)).toBeNull()
  })

  it('returns null for a degenerate (zero-area) quad', () => {
    const result = { model: { content: [5, 5, 5, 5, 5, 5, 5, 5] } }
    expect(extractRectFromCdpResult('DOM.getBoxModel', result)).toBeNull()
  })

  it('parses an unambiguous rect-shaped Runtime.callFunctionOn value', () => {
    const result = { result: { type: 'object', value: { x: 4, y: 8, width: 16, height: 32 } } }
    expect(extractRectFromCdpResult('Runtime.callFunctionOn', result)).toEqual({
      x: 4, y: 8, width: 16, height: 32,
    })
  })

  it('ignores a Runtime.callFunctionOn value with extra keys (ambiguous shape)', () => {
    const result = {
      result: { type: 'object', value: { x: 4, y: 8, width: 16, height: 32, tagName: 'DIV' } },
    }
    expect(extractRectFromCdpResult('Runtime.callFunctionOn', result)).toBeNull()
  })

  it('ignores a Runtime.callFunctionOn value missing rect keys', () => {
    const result = { result: { type: 'object', value: { foo: 'bar' } } }
    expect(extractRectFromCdpResult('Runtime.callFunctionOn', result)).toBeNull()
  })

  it('ignores a Runtime.callFunctionOn primitive value (a typical read-path eval result)', () => {
    const result = { result: { type: 'string', value: 'hello world' } }
    expect(extractRectFromCdpResult('Runtime.callFunctionOn', result)).toBeNull()
  })

  it('ignores a Runtime.callFunctionOn array value', () => {
    const result = { result: { type: 'object', value: [1, 2, 3, 4] } }
    expect(extractRectFromCdpResult('Runtime.callFunctionOn', result)).toBeNull()
  })

  it('returns null for a degenerate Runtime.callFunctionOn rect', () => {
    const result = { result: { type: 'object', value: { x: 0, y: 0, width: 0, height: 10 } } }
    expect(extractRectFromCdpResult('Runtime.callFunctionOn', result)).toBeNull()
  })

  it('returns null for an unrelated method', () => {
    expect(extractRectFromCdpResult('DOM.describeNode', { node: {} })).toBeNull()
  })

  it('returns null when result is not an object', () => {
    expect(extractRectFromCdpResult('DOM.getBoxModel', null)).toBeNull()
    expect(extractRectFromCdpResult('DOM.getBoxModel', 'nope')).toBeNull()
  })
})

function makeRegistration(): CdpProxyRegistration {
  return { pendingRectRequests: new Map() } as CdpProxyRegistration
}

describe('recordPendingRectRequest', () => {
  it('records an id -> method entry', () => {
    const registration = makeRegistration()
    recordPendingRectRequest(registration, 1, 'DOM.getBoxModel')
    expect(registration.pendingRectRequests.get(1)).toBe('DOM.getBoxModel')
  })

  it('drops the oldest entry once the cap is exceeded', () => {
    const registration = makeRegistration()
    for (let id = 1; id <= 33; id++) {
      recordPendingRectRequest(registration, id, 'Runtime.callFunctionOn')
    }
    expect(registration.pendingRectRequests.size).toBe(32)
    expect(registration.pendingRectRequests.has(1)).toBe(false)
    expect(registration.pendingRectRequests.has(2)).toBe(true)
    expect(registration.pendingRectRequests.has(33)).toBe(true)
  })
})

describe('isMutatingIntentCommand', () => {
  const cases: Array<{ command: string; mutating: boolean }> = [
    { command: 'click', mutating: true },
    { command: 'fill', mutating: true },
    { command: 'type', mutating: true },
    { command: 'select', mutating: true },
    { command: 'scroll', mutating: false },
    { command: 'snapshot', mutating: false },
    { command: 'eval', mutating: false },
    { command: 'get', mutating: false },
    { command: 'wait', mutating: false },
  ]
  for (const { command, mutating } of cases) {
    it(`${command} -> ${mutating}`, () => {
      expect(isMutatingIntentCommand(command)).toBe(mutating)
    })
  }
})
