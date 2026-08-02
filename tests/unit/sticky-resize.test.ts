import { describe, expect, it } from 'vitest'
import { stickyResizePatch } from '../../src/renderer/above-view/stickyResize'

const start = { width: 200, textSize: 14 }
const patch = { width: 400, height: 999, canvasX: 10, canvasY: 20 }

describe('stickyResizePatch', () => {
  it('never writes height — the measurement owns that axis', () => {
    for (const handle of ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw'] as const) {
      expect(stickyResizePatch(handle, start, patch)).not.toHaveProperty('height')
    }
  })

  it('reflows on side handles: width only, text size untouched', () => {
    expect(stickyResizePatch('e', start, patch)).toEqual({ width: 400, canvasX: 10, canvasY: 20 })
    expect(stickyResizePatch('w', start, patch)).not.toHaveProperty('textSize')
  })

  it('scales text with the width ratio on corner and vertical handles', () => {
    expect(stickyResizePatch('se', start, patch).textSize).toBe(28)
    expect(stickyResizePatch('n', start, patch).textSize).toBe(28)
  })

  it('clamps the scaled text size', () => {
    expect(stickyResizePatch('se', start, { ...patch, width: 1 }).textSize).toBe(8)
  })
})
