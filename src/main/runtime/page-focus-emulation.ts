/**
 * Focus emulation for the keyboard-target page.
 *
 * A page renders offscreen, so its widget host's `Focus()` is a no-op: without
 * help it never believes it has focus, which means no caret, no active
 * selection color, and no `:focus-within`. `Emulation.setFocusEmulationEnabled`
 * is the one lever that makes it believe, and it rides the page's single shared
 * CDP session.
 *
 * Exactly one page is emulated at a time — the page keys are being forwarded
 * to. aboveView keeps OS keyboard focus regardless (see
 * `focus-reconciler-runtime.ts`); emulation plus forwarding is how the page
 * behaves as though focus were its own.
 */

import { findPageById } from './runtime-context'
import type { Page } from './runtime-entities'
import { ensurePageDebugger } from './page-debugger'
import { currentKeyboardTargetPageId } from './selection-controller'

/** The page emulation is currently enabled on, as far as main knows. */
let emulatedPageId: string | null = null

/**
 * A detached session drops the override with it, so the record is forgotten and
 * the next pass re-applies rather than skipping as a no-op. One shared handler
 * rather than a closure per call, which would grow the session's detach-handler
 * set on every target change.
 */
const forgetEmulation = (): void => {
  emulatedPageId = null
}

function setEmulation(page: Page, enabled: boolean): boolean {
  const wc = page.host.webContents
  if (wc.isDestroyed()) return false
  if (!ensurePageDebugger(wc, forgetEmulation)) return false
  wc.debugger
    .sendCommand('Emulation.setFocusEmulationEnabled', { enabled })
    .catch(() => {
      if (emulatedPageId === page.id) emulatedPageId = null
    })
  return true
}

function moveEmulationTo(pageId: string | null): void {
  if (emulatedPageId === pageId) return
  const previous = emulatedPageId ? findPageById(emulatedPageId) : null
  if (previous) setEmulation(previous, false)
  emulatedPageId = null
  if (!pageId) return
  const next = findPageById(pageId)
  if (!next) return
  if (setEmulation(next, true)) emulatedPageId = pageId
}

/**
 * Point emulation at whichever page owns the keyboard. Called from
 * `layoutAllViews()` right after `reconcileFocus()`, which reads the same
 * predicate.
 */
export function reconcilePageFocusEmulation(): void {
  moveEmulationTo(currentKeyboardTargetPageId())
}

/**
 * Emulate focus on a page the pointer just pressed into, ahead of the layout
 * pass. A synthesized mouseDown selects text, and without focus Chromium paints
 * that selection in its inactive gray.
 *
 * Only the keyboard-target page qualifies: a press forwarded for the inspect
 * eyedropper is not a text interaction and must not move focus.
 */
export function ensurePageFocusEmulated(pageId: string): void {
  if (currentKeyboardTargetPageId() !== pageId) return
  moveEmulationTo(pageId)
}

/**
 * Wire a freshly created page. A navigation or renderer crash starts a new
 * renderer that never saw the override, so the record is forgotten and the next
 * pass re-applies rather than trusting it.
 */
export function registerPageFocusEmulation(page: Page): void {
  const wc = page.host.webContents
  const reapply = (): void => {
    if (emulatedPageId !== page.id) return
    emulatedPageId = null
    moveEmulationTo(page.id)
  }
  wc.on('did-navigate', reapply)
  wc.on('render-process-gone', reapply)
  wc.once('destroyed', () => {
    if (emulatedPageId === page.id) emulatedPageId = null
  })
}
