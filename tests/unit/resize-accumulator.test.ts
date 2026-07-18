import { describe, expect, it } from 'vitest'
import {
  applyCornerDelta,
  applyEdgeDelta,
  applyHandleDelta,
  startResize,
  type ResizeAccumulator,
  type ResizeConfig,
} from '../../src/shared/resize-accumulator'

const FREE: ResizeConfig = { minWidth: 10, minHeight: 10, aspectRatioResizeMode: 'off' }
const SHIFT_LOCKS: ResizeConfig = { ...FREE, aspectRatioResizeMode: 'shift-locks' }
const SHIFT_UNLOCKS: ResizeConfig = { ...FREE, aspectRatioResizeMode: 'shift-unlocks' }

function fresh(): ResizeAccumulator {
  return startResize({ width: 200, height: 100, canvasX: 0, canvasY: 0 })
}

describe('resize-accumulator', () => {
  describe('startResize', () => {
    it('captures aspect ratio at start', () => {
      const acc = startResize({ width: 200, height: 100, canvasX: 50, canvasY: 25 })
      expect(acc.aspect).toBe(2)
      expect(acc.canvasX).toBe(50)
      expect(acc.canvasY).toBe(25)
    })

    it('defaults aspect to 1 when height is 0', () => {
      const acc = startResize({ width: 200, height: 0, canvasX: 0, canvasY: 0 })
      expect(acc.aspect).toBe(1)
    })
  })

  describe('applyCornerDelta', () => {
    it('keeps the opposite edges fixed while rounding a top-left resize', () => {
      const acc = startResize({ width: 99.8, height: 79.8, canvasX: 0.4, canvasY: 0.4 })
      const fixedRight = Math.round(acc.canvasX + acc.width)
      const fixedBottom = Math.round(acc.canvasY + acc.height)
      const patch = applyCornerDelta(
        acc,
        'top-left',
        { screenDx: 0.2, screenDy: 0.2, zoom: 1, shiftKey: false },
        FREE,
      )

      expect((patch.canvasX ?? 0) + patch.width).toBe(fixedRight)
      expect((patch.canvasY ?? 0) + patch.height).toBe(fixedBottom)
    })

    it('bottom-right grows width and height by the screen delta divided by zoom', () => {
      const acc = fresh()
      const patch = applyCornerDelta(
        acc,
        'bottom-right',
        { screenDx: 50, screenDy: 30, zoom: 1, shiftKey: false },
        FREE,
      )
      expect(patch.width).toBe(250)
      expect(patch.height).toBe(130)
      // No origin movement on bottom-right.
      expect(patch.canvasX).toBeUndefined()
      expect(patch.canvasY).toBeUndefined()
    })

    it('top-left shrinks and shifts canvasX/canvasY toward the cursor', () => {
      const acc = fresh()
      const patch = applyCornerDelta(
        acc,
        'top-left',
        { screenDx: 20, screenDy: 10, zoom: 1, shiftKey: false },
        FREE,
      )
      // Dragging top-left toward bottom-right shrinks both dims; flipX=-1, flipY=-1.
      expect(patch.width).toBe(180)
      expect(patch.height).toBe(90)
      // Origin moves by the clamped delta (= -dx*flipX, -dy*flipY -> +dx, +dy).
      expect(patch.canvasX).toBe(20)
      expect(patch.canvasY).toBe(10)
    })

    it('clamps to minWidth/minHeight', () => {
      const acc = startResize({ width: 30, height: 30, canvasX: 0, canvasY: 0 })
      const patch = applyCornerDelta(
        acc,
        'bottom-right',
        { screenDx: -100, screenDy: -100, zoom: 1, shiftKey: false },
        FREE,
      )
      expect(patch.width).toBe(10)
      expect(patch.height).toBe(10)
    })

    it('shift-locks: aspect locked when shift held', () => {
      const acc = fresh() // 2:1 aspect
      const patch = applyCornerDelta(
        acc,
        'bottom-right',
        { screenDx: 100, screenDy: 0, zoom: 1, shiftKey: true },
        SHIFT_LOCKS,
      )
      // dx=100 grows width to 300; with aspect lock 2:1 height becomes 150.
      expect(patch.width).toBe(300)
      expect(patch.height).toBe(150)
    })

    it('shift-unlocks: aspect locked by default, free when shift held', () => {
      const acc = fresh() // 2:1 aspect
      const lockedPatch = applyCornerDelta(
        acc,
        'bottom-right',
        { screenDx: 100, screenDy: 0, zoom: 1, shiftKey: false },
        SHIFT_UNLOCKS,
      )
      // Without shift, locked; height follows.
      expect(lockedPatch.width).toBe(300)
      expect(lockedPatch.height).toBe(150)

      const acc2 = fresh()
      const freePatch = applyCornerDelta(
        acc2,
        'bottom-right',
        { screenDx: 100, screenDy: 0, zoom: 1, shiftKey: true },
        SHIFT_UNLOCKS,
      )
      expect(freePatch.width).toBe(300)
      // With shift, unlocked; height stays at start value.
      expect(freePatch.height).toBe(100)
    })

    it('zoom divides screen pixels into canvas pixels', () => {
      const acc = fresh()
      const patch = applyCornerDelta(
        acc,
        'bottom-right',
        { screenDx: 100, screenDy: 50, zoom: 2, shiftKey: false },
        FREE,
      )
      expect(patch.width).toBe(250)
      expect(patch.height).toBe(125)
    })
  })

  describe('applyEdgeDelta', () => {
    it('keeps the opposite edge fixed while rounding a left resize', () => {
      const acc = startResize({ width: 99.8, height: 100, canvasX: 0.4, canvasY: 0 })
      const fixedRight = Math.round(acc.canvasX + acc.width)
      const patch = applyEdgeDelta(
        acc,
        'left',
        { screenDx: 0.2, screenDy: 0, zoom: 1, shiftKey: false },
        FREE,
      )

      expect((patch.canvasX ?? 0) + patch.width).toBe(fixedRight)
    })

    it('keeps the opposite edge fixed while rounding a top resize', () => {
      const acc = startResize({ width: 100, height: 99.8, canvasX: 0, canvasY: 0.4 })
      const fixedBottom = Math.round(acc.canvasY + acc.height)
      const patch = applyEdgeDelta(
        acc,
        'top',
        { screenDx: 0, screenDy: 0.2, zoom: 1, shiftKey: false },
        FREE,
      )

      expect((patch.canvasY ?? 0) + patch.height).toBe(fixedBottom)
    })

    it('right edge moves width only', () => {
      const acc = fresh()
      const patch = applyEdgeDelta(
        acc,
        'right',
        { screenDx: 40, screenDy: 80, zoom: 1, shiftKey: false },
        FREE,
      )
      expect(patch.width).toBe(240)
      expect(patch.height).toBe(100)
      expect(patch.canvasX).toBeUndefined()
    })

    it('left edge moves canvasX', () => {
      const acc = fresh()
      const patch = applyEdgeDelta(
        acc,
        'left',
        { screenDx: 30, screenDy: 0, zoom: 1, shiftKey: false },
        FREE,
      )
      // Dragging left edge right by 30 shrinks width by 30 and pushes origin by 30.
      expect(patch.width).toBe(170)
      expect(patch.canvasX).toBe(30)
    })

    it('top edge moves canvasY', () => {
      const acc = fresh()
      const patch = applyEdgeDelta(
        acc,
        'top',
        { screenDx: 0, screenDy: 20, zoom: 1, shiftKey: false },
        FREE,
      )
      expect(patch.height).toBe(80)
      expect(patch.canvasY).toBe(20)
    })

    it('aspect lock on horizontal edge updates orthogonal dimension', () => {
      const acc = fresh() // 2:1
      const patch = applyEdgeDelta(
        acc,
        'right',
        { screenDx: 100, screenDy: 0, zoom: 1, shiftKey: true },
        SHIFT_LOCKS,
      )
      expect(patch.width).toBe(300)
      expect(patch.height).toBe(150)
    })
  })

  describe('applyHandleDelta', () => {
    it('routes nw/ne/se/sw to corner deltas', () => {
      const acc = fresh()
      const cornerPatch = applyCornerDelta(
        startResize({ width: 200, height: 100, canvasX: 0, canvasY: 0 }),
        'top-right',
        { screenDx: 20, screenDy: 10, zoom: 1, shiftKey: false },
        FREE,
      )
      const handlePatch = applyHandleDelta(
        acc,
        'ne',
        { screenDx: 20, screenDy: 10, zoom: 1, shiftKey: false },
        FREE,
      )
      expect(handlePatch).toEqual(cornerPatch)
    })

    it('routes n/s/e/w to edge deltas', () => {
      const acc = fresh()
      const edgePatch = applyEdgeDelta(
        startResize({ width: 200, height: 100, canvasX: 0, canvasY: 0 }),
        'bottom',
        { screenDx: 0, screenDy: 25, zoom: 1, shiftKey: false },
        FREE,
      )
      const handlePatch = applyHandleDelta(
        acc,
        's',
        { screenDx: 0, screenDy: 25, zoom: 1, shiftKey: false },
        FREE,
      )
      expect(handlePatch).toEqual(edgePatch)
    })
  })

  describe('accumulator mutation across ticks', () => {
    it('accumulates deltas and reflects them in subsequent patches', () => {
      const acc = fresh()
      applyCornerDelta(acc, 'bottom-right', { screenDx: 30, screenDy: 0, zoom: 1, shiftKey: false }, FREE)
      const second = applyCornerDelta(
        acc,
        'bottom-right',
        { screenDx: 20, screenDy: 0, zoom: 1, shiftKey: false },
        FREE,
      )
      // First tick widened to 230; second adds another 20 → 250.
      expect(second.width).toBe(250)
    })
  })
})
