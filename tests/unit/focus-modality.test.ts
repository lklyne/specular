import { describe, expect, it } from 'vitest'
import { keyMovesFocus } from '../../src/renderer/shared/focusModality'

describe('keyMovesFocus', () => {
  it('counts the keys that move focus', () => {
    for (const key of ['Tab', 'ArrowDown', 'ArrowUp', 'ArrowLeft', 'ArrowRight', 'Home', 'End']) {
      expect(keyMovesFocus(key)).toBe(true)
    }
  })

  it('does not count typing, so a click into a field then typing stays pointer-driven', () => {
    for (const key of ['a', 'Z', ' ', '1', 'Enter', 'Backspace', 'Shift']) {
      expect(keyMovesFocus(key)).toBe(false)
    }
  })
})
