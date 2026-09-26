/**
 * A focus session resizes its page (fit / fill / device), so every change to
 * the session's target or mode must ask for a scene rebuild — the camera move
 * that follows carries none. Without it a Fit → Fill click left the page at
 * its old size and the focus bar on the old mode until something unrelated
 * dirtied the canvas.
 *
 * Mutation-verified by: emptying `sessionGeometryChanged()` — all four cases
 * fail.
 */

import { beforeEach, describe, expect, it } from 'vitest'
import {
  beginFocusSession,
  endFocusSession,
  repointFocusSession,
  setFocusSessionMode,
} from '../../src/main/runtime/focus-session'
import { consumeDirty } from '../../src/main/runtime/layout-dirty'

const begin = () =>
  beginFocusSession({ target: { kind: 'page', id: 'a' }, mode: 'fit', annotationsVisible: false })

describe('focus session dirties the scene', () => {
  beforeEach(() => {
    endFocusSession('dismiss')
    consumeDirty('canvas')
  })

  it('on entry', () => {
    begin()
    expect(consumeDirty('canvas')).toBe(true)
  })

  it('on a mode switch', () => {
    begin()
    consumeDirty('canvas')
    setFocusSessionMode('fill')
    expect(consumeDirty('canvas')).toBe(true)
  })

  it('on a switch to another page', () => {
    begin()
    consumeDirty('canvas')
    repointFocusSession('b')
    expect(consumeDirty('canvas')).toBe(true)
  })

  it('on exit, and not when there was no session to end', () => {
    begin()
    consumeDirty('canvas')
    endFocusSession('dismiss')
    expect(consumeDirty('canvas')).toBe(true)
    endFocusSession('dismiss')
    expect(consumeDirty('canvas')).toBe(false)
  })
})
