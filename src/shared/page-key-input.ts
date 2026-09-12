/**
 * Keyboard input for a page that renders offscreen.
 *
 * An offscreen widget host's `Focus()` is a no-op and OS key events go to the
 * focused native view, so a page never receives a keystroke of its own. Keys
 * reach it as synthesized CDP `Input.dispatchKeyEvent` calls built from a DOM
 * KeyboardEvent that landed somewhere else (aboveView's keyboard sink).
 *
 * The mapping tables are here rather than in main because the payload is
 * authored in a renderer and dispatched in main, and both halves have to agree
 * on what a key is.
 */

/** One DOM key event, flattened for the process boundary. */
export interface ForwardKeyPayload {
  kind: 'down' | 'up'
  key: string
  code: string
  /** Printable text this press inserts, when there is any. */
  text: string | null
  repeat: boolean
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}

/** Windows virtual key code for the keys CDP needs one for. */
function windowsVirtualKeyCodeFor(key: string): number | null {
  const named: Record<string, number> = {
    Backspace: 8,
    Tab: 9,
    Enter: 13,
    Shift: 16,
    Control: 17,
    Alt: 18,
    Escape: 27,
    ' ': 32,
    PageUp: 33,
    PageDown: 34,
    End: 35,
    Home: 36,
    ArrowLeft: 37,
    ArrowUp: 38,
    ArrowRight: 39,
    ArrowDown: 40,
    Delete: 46,
    Meta: 91,
  }
  if (key in named) return named[key]
  if (key.length === 1) {
    const upper = key.toUpperCase()
    const code = upper.charCodeAt(0)
    if ((code >= 48 && code <= 57) || (code >= 65 && code <= 90)) return code
  }
  return null
}

/** CDP `Input.*` modifier bitmask: Alt=1, Ctrl=2, Meta=4, Shift=8. */
function cdpModifiersFor(mods: {
  shiftKey: boolean
  ctrlKey: boolean
  altKey: boolean
  metaKey: boolean
}): number {
  return (mods.altKey ? 1 : 0) | (mods.ctrlKey ? 2 : 0) | (mods.metaKey ? 4 : 0) | (mods.shiftKey ? 8 : 0)
}

/**
 * Params for one `Input.dispatchKeyEvent`.
 *
 * A press that inserts text is a `keyDown` carrying that text; one that does
 * not is a `rawKeyDown`, because a `keyDown` with no text suppresses the
 * `keypress`-equivalent and Chromium treats it as a dead key.
 */
export function cdpKeyEventParams(payload: ForwardKeyPayload): Record<string, unknown> {
  const virtualKey = windowsVirtualKeyCodeFor(payload.key)
  const base: Record<string, unknown> = {
    key: payload.key,
    code: payload.code,
    modifiers: cdpModifiersFor(payload),
    autoRepeat: payload.repeat,
    ...(virtualKey !== null
      ? { windowsVirtualKeyCode: virtualKey, nativeVirtualKeyCode: virtualKey }
      : {}),
  }
  if (payload.kind === 'up') return { ...base, type: 'keyUp' }
  return {
    ...base,
    type: payload.text ? 'keyDown' : 'rawKeyDown',
    ...(payload.text ? { text: payload.text, unmodifiedText: payload.text } : {}),
  }
}

/** The text a key press inserts, as the sink reads it off a DOM event. */
export function textForKeyPress(event: {
  key: string
  ctrlKey: boolean
  metaKey: boolean
}): string | null {
  if (event.key.length === 1 && !event.ctrlKey && !event.metaKey) return event.key
  if (event.key === 'Enter') return '\r'
  return null
}
