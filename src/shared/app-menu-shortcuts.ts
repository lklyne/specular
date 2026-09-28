import { acceleratorString, bindingById } from './bindings'

/**
 * App-menu shortcuts that stay with the app while a page owns the keyboard —
 * the keys a browser reserves from its tabs (Quit, Close Tab, Hide).
 *
 * A page renders offscreen and never takes part in menu matching, so these
 * are the keys aboveView's keyboard sink leaves for the menu instead of
 * forwarding. The Edit menu's are absent on purpose: a page gets first claim
 * on Cmd+C/V/Z like any web page, and its editor resolves them itself.
 *
 * Role items carry these explicitly rather than relying on Electron's role
 * defaults, so this table is the whole truth. `minimize` belongs to the
 * `windowMenu` role, whose items can't be given one, so it restates the
 * default.
 */
export const APP_MENU_ACCELERATORS = {
  settings: 'CmdOrCtrl+,',
  hide: 'Command+H',
  hideOthers: 'Command+Alt+H',
  quit: 'CmdOrCtrl+Q',
  closeTab: acceleratorString(bindingById('close-tab').defaultKey),
  toggleCanvasDevTools: 'CmdOrCtrl+Alt+I',
  toggleSelectedPageDevTools: 'CmdOrCtrl+Alt+Shift+I',
  openMotionDebugWindow: 'CmdOrCtrl+Shift+D',
  togglePerfTrace: 'CmdOrCtrl+Alt+Shift+P',
  toggleFullScreen: 'Control+Command+F',
  minimize: 'CmdOrCtrl+M',
} as const

interface ParsedAccelerator {
  code: string
  meta: boolean
  ctrl: boolean
  alt: boolean
  shift: boolean
}

// Physical key, not `event.key`: with Option held macOS turns a letter into
// another character (Option+I is a dead key), so the character can't match.
const PUNCTUATION_CODES: Record<string, string> = { ',': 'Comma' }

function codeFor(key: string): string {
  if (/^[A-Z]$/i.test(key)) return `Key${key.toUpperCase()}`
  const code = PUNCTUATION_CODES[key]
  if (!code) throw new Error(`No key code for accelerator key "${key}"`)
  return code
}

function parseAccelerator(accelerator: string, isMac: boolean): ParsedAccelerator {
  const parsed: ParsedAccelerator = { code: '', meta: false, ctrl: false, alt: false, shift: false }
  for (const part of accelerator.split('+')) {
    switch (part) {
      case 'CmdOrCtrl':
        if (isMac) parsed.meta = true
        else parsed.ctrl = true
        break
      case 'Command':
        parsed.meta = true
        break
      case 'Control':
        parsed.ctrl = true
        break
      case 'Alt':
        parsed.alt = true
        break
      case 'Shift':
        parsed.shift = true
        break
      default:
        parsed.code = codeFor(part)
    }
  }
  return parsed
}

const parsedFor = new Map<boolean, ParsedAccelerator[]>()

function parsedAccelerators(isMac: boolean): ParsedAccelerator[] {
  let parsed = parsedFor.get(isMac)
  if (!parsed) {
    parsed = Object.values(APP_MENU_ACCELERATORS).map((a) => parseAccelerator(a, isMac))
    parsedFor.set(isMac, parsed)
  }
  return parsed
}

/** Whether a key event is one of the app menu's reserved shortcuts. */
export function isAppMenuShortcut(
  event: {
    code: string
    metaKey: boolean
    ctrlKey: boolean
    altKey: boolean
    shiftKey: boolean
  },
  isMac: boolean,
): boolean {
  return parsedAccelerators(isMac).some(
    (a) =>
      a.code === event.code &&
      a.meta === event.metaKey &&
      a.ctrl === event.ctrlKey &&
      a.alt === event.altKey &&
      a.shift === event.shiftKey,
  )
}
