/**
 * Which keys aboveView's keyboard sink leaves for the app menu instead of
 * forwarding into a page.
 *
 * Protects two things: a page keeps first claim on its editing keys (a code
 * editor's own Cmd+Z), and the app keeps Quit / Close Tab even while a page
 * owns the keyboard. Also parses every accelerator in the table, so one the
 * matcher can't read fails here rather than silently reserving nothing.
 *
 * Mutation-verified by adding 'CmdOrCtrl+Z' to APP_MENU_ACCELERATORS (the
 * editing case fails) and by dropping the meta comparison (the bare-letter
 * case fails).
 */

import { describe, it, expect } from 'vitest'
import { isAppMenuShortcut } from '../../src/shared/app-menu-shortcuts'

function key(code: string, mods: { meta?: boolean; ctrl?: boolean; alt?: boolean; shift?: boolean } = {}) {
  return {
    code,
    metaKey: mods.meta ?? false,
    ctrlKey: mods.ctrl ?? false,
    altKey: mods.alt ?? false,
    shiftKey: mods.shift ?? false,
  }
}

describe('isAppMenuShortcut', () => {
  it('reserves the app-level shortcuts', () => {
    expect(isAppMenuShortcut(key('KeyQ', { meta: true }), true)).toBe(true)
    expect(isAppMenuShortcut(key('KeyW', { meta: true }), true)).toBe(true)
    expect(isAppMenuShortcut(key('Comma', { meta: true }), true)).toBe(true)
    expect(isAppMenuShortcut(key('KeyF', { meta: true, ctrl: true }), true)).toBe(true)
    // Option turns I into a dead key, so matching has to go by physical key.
    expect(isAppMenuShortcut(key('KeyI', { meta: true, alt: true }), true)).toBe(true)
  })

  it('leaves editing keys to the page', () => {
    for (const code of ['KeyC', 'KeyV', 'KeyX', 'KeyA', 'KeyZ']) {
      expect(isAppMenuShortcut(key(code, { meta: true }), true)).toBe(false)
    }
    expect(isAppMenuShortcut(key('KeyZ', { meta: true, shift: true }), true)).toBe(false)
  })

  it('leaves bare letters and bare modifiers to the page', () => {
    expect(isAppMenuShortcut(key('KeyQ'), true)).toBe(false)
    expect(isAppMenuShortcut(key('MetaLeft', { meta: true }), true)).toBe(false)
    expect(isAppMenuShortcut(key('ShiftLeft', { shift: true }), true)).toBe(false)
  })

  it('needs the exact modifiers', () => {
    expect(isAppMenuShortcut(key('KeyQ', { meta: true, shift: true }), true)).toBe(false)
  })

  it('reads CmdOrCtrl as Ctrl off macOS', () => {
    expect(isAppMenuShortcut(key('KeyQ', { ctrl: true }), false)).toBe(true)
    expect(isAppMenuShortcut(key('KeyQ', { meta: true }), false)).toBe(false)
  })
})
