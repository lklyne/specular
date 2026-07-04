import { describe, expect, it } from 'vitest'
import { DRAG_THRESHOLD } from '../../src/shared/gesture-utils'
import {
  beginPressGesture,
  pressGestureIgnoresBlur,
  pressGestureStep,
  type PressGestureState,
} from '../../src/shared/press-gesture'

function armed(startX = 100, startY = 100): PressGestureState {
  return beginPressGesture(startX, startY)
}

function promoted(startX = 100, startY = 100): PressGestureState {
  const step = pressGestureStep(armed(startX, startY), {
    type: 'move',
    x: startX + DRAG_THRESHOLD,
    y: startY,
  })
  expect(step.outcome).toBe('promote-to-drag')
  return step.state
}

describe('pressGestureStep — threshold promotion', () => {
  it('move below the threshold on both axes keeps the press armed', () => {
    const step = pressGestureStep(armed(), {
      type: 'move',
      x: 100 + DRAG_THRESHOLD - 1,
      y: 100 + DRAG_THRESHOLD - 1,
    })
    expect(step.outcome).toBe('ignore')
    expect(step.state.dragging).toBe(false)
  })

  it('a fractional hair below the threshold still ignores', () => {
    const step = pressGestureStep(armed(), {
      type: 'move',
      x: 100 + DRAG_THRESHOLD - 0.001,
      y: 100,
    })
    expect(step.outcome).toBe('ignore')
  })

  it('move exactly at the threshold promotes (boundary is inclusive)', () => {
    const step = pressGestureStep(armed(), {
      type: 'move',
      x: 100 + DRAG_THRESHOLD,
      y: 100,
    })
    expect(step.outcome).toBe('promote-to-drag')
    expect(step.state.dragging).toBe(true)
  })

  it('move past the threshold promotes', () => {
    const step = pressGestureStep(armed(), { type: 'move', x: 150, y: 150 })
    expect(step.outcome).toBe('promote-to-drag')
  })

  it('either axis alone reaching the threshold promotes (y only)', () => {
    const step = pressGestureStep(armed(), {
      type: 'move',
      x: 100,
      y: 100 + DRAG_THRESHOLD,
    })
    expect(step.outcome).toBe('promote-to-drag')
  })

  it('negative displacement counts the same as positive', () => {
    const step = pressGestureStep(armed(), {
      type: 'move',
      x: 100 - DRAG_THRESHOLD,
      y: 100,
    })
    expect(step.outcome).toBe('promote-to-drag')
  })

  it('displacement measures from the down point, not per-move deltas', () => {
    // Wander below threshold, return to origin, then cross: only the last
    // move promotes — the intermediate positions never accumulate.
    let state = armed()
    let step = pressGestureStep(state, { type: 'move', x: 103, y: 100 })
    expect(step.outcome).toBe('ignore')
    step = pressGestureStep(step.state, { type: 'move', x: 100, y: 100 })
    expect(step.outcome).toBe('ignore')
    step = pressGestureStep(step.state, { type: 'move', x: 100, y: 104 })
    expect(step.outcome).toBe('promote-to-drag')
  })

  it('a move after promotion is ignored (the shell has handed off the session)', () => {
    const step = pressGestureStep(promoted(), { type: 'move', x: 500, y: 500 })
    expect(step.outcome).toBe('ignore')
    expect(step.state.dragging).toBe(true)
  })
})

describe('pressGestureStep — release and cancel outcomes', () => {
  it('stationary release commits the press', () => {
    expect(pressGestureStep(armed(), { type: 'up' }).outcome).toBe('commit-press')
  })

  it('release after promotion ends the drag', () => {
    expect(pressGestureStep(promoted(), { type: 'up' }).outcome).toBe('end-drag')
  })

  it('cancel while armed discards silently (nothing to unwind)', () => {
    expect(pressGestureStep(armed(), { type: 'cancel' }).outcome).toBe('ignore')
  })

  it('cancel after promotion ends the drag', () => {
    expect(pressGestureStep(promoted(), { type: 'cancel' }).outcome).toBe('end-drag')
  })
})

describe('pressGestureIgnoresBlur — phantom-blur guard (§4.6)', () => {
  it('swallows blur while the press is armed (pre-threshold focus-reconciler phantom)', () => {
    expect(pressGestureIgnoresBlur(armed())).toBe(true)
  })

  it('still swallows blur after below-threshold movement', () => {
    const step = pressGestureStep(armed(), { type: 'move', x: 102, y: 101 })
    expect(pressGestureIgnoresBlur(step.state)).toBe(true)
  })

  it('lets blur cancel once promoted to a drag', () => {
    expect(pressGestureIgnoresBlur(promoted())).toBe(false)
  })
})
