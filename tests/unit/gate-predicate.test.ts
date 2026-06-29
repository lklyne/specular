import { describe, it, expect } from 'vitest'
import { shouldGateBeOpen, type GateInputs } from '../../src/main/runtime/gate-predicate'

function base(): GateInputs {
  return {
    activeTool: { kind: 'select' },
    commentOverlayActive: false,
  }
}

describe('shouldGateBeOpen', () => {
  it('is open by default in the single canvas view', () => {
    expect(shouldGateBeOpen(base())).toBe(true)
  })

  it('is open for drawing and commenting tools', () => {
    expect(shouldGateBeOpen({ ...base(), activeTool: { kind: 'draw' } })).toBe(true)
    expect(shouldGateBeOpen({ ...base(), activeTool: { kind: 'comment' } })).toBe(true)
  })

  it('is closed for inspect until a composer/overlay is active', () => {
    expect(shouldGateBeOpen({ ...base(), activeTool: { kind: 'inspect' } })).toBe(false)
    expect(
      shouldGateBeOpen({
        ...base(),
        activeTool: { kind: 'inspect' },
        commentOverlayActive: true,
      }),
    ).toBe(true)
  })
})
