import { describe, expect, it } from 'vitest'
import { textResizePatch } from '../../src/renderer/above-view/textResize'

const start = { width: 200, textSize: 14 }
const patch = { width: 400, height: 999, canvasX: 10, canvasY: 20 }

describe('textResizePatch', () => {
  it('never writes height — the measurement owns that axis', () => {
    for (const handle of ['n', 's', 'e', 'w', 'ne', 'nw', 'se', 'sw'] as const) {
      expect(textResizePatch(handle, start, patch)).not.toHaveProperty('height')
    }
  })

  it('reflows on side handles: width only, text size untouched', () => {
    expect(textResizePatch('e', start, patch)).toEqual({ width: 400, canvasX: 10, canvasY: 20 })
    expect(textResizePatch('w', start, patch)).not.toHaveProperty('textSize')
  })

  it('scales the text with the width ratio on corner and vertical handles', () => {
    expect(textResizePatch('se', start, patch).textSize).toBe(28)
    expect(textResizePatch('n', start, patch).textSize).toBe(28)
  })

  it('clamps the scaled text size', () => {
    expect(textResizePatch('se', start, { ...patch, width: 1 }).textSize).toBe(8)
  })
})
