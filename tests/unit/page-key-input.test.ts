/**
 * CDP key-event params for a page that renders offscreen.
 *
 * The keyDown/rawKeyDown split is the load-bearing part: a `keyDown` with no
 * `text` is a dead key to Chromium, so a non-printable press has to be a
 * `rawKeyDown` instead, and a printable one has to carry its text.
 *
 * Mutation-verified by making the down branch always emit `keyDown` (the
 * non-printable cases fail) and by dropping the `windowsVirtualKeyCode` spread
 * (the Enter and Escape cases fail).
 */

import { describe, it, expect } from 'vitest'
import { cdpKeyEventParams, type ForwardKeyPayload } from '../../src/shared/page-key-input'

function payload(over: Partial<ForwardKeyPayload> = {}): ForwardKeyPayload {
  return {
    kind: 'down',
    key: 'a',
    code: 'KeyA',
    text: 'a',
    repeat: false,
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: false,
    ...over,
  }
}

describe('cdpKeyEventParams', () => {
  it('sends a printable press as keyDown carrying its text', () => {
    expect(cdpKeyEventParams(payload())).toMatchObject({
      type: 'keyDown',
      key: 'a',
      code: 'KeyA',
      text: 'a',
      unmodifiedText: 'a',
      windowsVirtualKeyCode: 65,
      nativeVirtualKeyCode: 65,
      modifiers: 0,
      autoRepeat: false,
    })
  })

  it('sends a non-printable press as rawKeyDown with no text', () => {
    const params = cdpKeyEventParams(
      payload({ key: 'Escape', code: 'Escape', text: null }),
    )
    expect(params.type).toBe('rawKeyDown')
    expect(params).not.toHaveProperty('text')
    expect(params).not.toHaveProperty('unmodifiedText')
    expect(params.windowsVirtualKeyCode).toBe(27)
  })

  it('sends a release as keyUp with no text, printable or not', () => {
    const params = cdpKeyEventParams(payload({ kind: 'up', text: null }))
    expect(params.type).toBe('keyUp')
    expect(params).not.toHaveProperty('text')
  })

  it('maps Enter to a carriage return and its virtual key', () => {
    expect(cdpKeyEventParams(payload({ key: 'Enter', code: 'Enter', text: '\r' }))).toMatchObject({
      type: 'keyDown',
      text: '\r',
      unmodifiedText: '\r',
      windowsVirtualKeyCode: 13,
    })
  })

  it('packs modifiers as the CDP bitmask (alt 1, ctrl 2, meta 4, shift 8)', () => {
    expect(cdpKeyEventParams(payload({ shiftKey: true })).modifiers).toBe(8)
    expect(cdpKeyEventParams(payload({ metaKey: true, altKey: true })).modifiers).toBe(5)
    expect(
      cdpKeyEventParams(
        payload({ shiftKey: true, ctrlKey: true, altKey: true, metaKey: true }),
      ).modifiers,
    ).toBe(15)
  })

  it('omits the virtual key for a key with no mapping', () => {
    const params = cdpKeyEventParams(payload({ key: 'F13', code: 'F13', text: null }))
    expect(params).not.toHaveProperty('windowsVirtualKeyCode')
    expect(params).not.toHaveProperty('nativeVirtualKeyCode')
  })

  it('carries autorepeat through', () => {
    expect(cdpKeyEventParams(payload({ repeat: true })).autoRepeat).toBe(true)
  })
})

describe('cdpKeyEventParams editing commands', () => {
  const press = (key: string, mods: Partial<ForwardKeyPayload> = {}): ForwardKeyPayload => ({
    kind: 'down',
    key,
    code: `Key${key.toUpperCase()}`,
    text: null,
    repeat: false,
    shiftKey: false,
    ctrlKey: false,
    altKey: false,
    metaKey: true,
    ...mods,
  })

  it('names the editing command for the clipboard shortcuts', () => {
    // Without this, macOS Chromium runs no editing action for a synthesized
    // press and Cmd+C in an entered page copies nothing.
    expect(cdpKeyEventParams(press('c')).commands).toEqual(['copy'])
    expect(cdpKeyEventParams(press('x')).commands).toEqual(['cut'])
    expect(cdpKeyEventParams(press('v')).commands).toEqual(['paste'])
    expect(cdpKeyEventParams(press('a')).commands).toEqual(['selectAll'])
  })

  it('distinguishes the shifted variants', () => {
    expect(cdpKeyEventParams(press('z')).commands).toEqual(['undo'])
    expect(cdpKeyEventParams(press('z', { shiftKey: true })).commands).toEqual(['redo'])
    expect(cdpKeyEventParams(press('v', { shiftKey: true })).commands).toEqual([
      'pasteAndMatchStyle',
    ])
  })

  it('leaves ordinary and non-Meta presses alone', () => {
    expect(cdpKeyEventParams(press('c', { metaKey: false })).commands).toBeUndefined()
    expect(cdpKeyEventParams(press('b')).commands).toBeUndefined()
    // Ctrl+Cmd+C is a different keystroke and means nothing to the editor.
    expect(cdpKeyEventParams(press('c', { ctrlKey: true })).commands).toBeUndefined()
  })

  it('carries no command on the key release', () => {
    expect(cdpKeyEventParams(press('c', { kind: 'up' })).commands).toBeUndefined()
  })
})
