/**
 * Page cursor bridge — mirror the keyboard-target page's `cursor-changed`
 * events onto aboveView's `<body>` CSS cursor so the OS shows the right
 * pointer (hand on links, I-beam over text, etc).
 *
 * The OS picks the cursor from the topmost WebContentsView at the pointer
 * location. With aboveView always covering the canvas in canvas mode, the
 * page's own cursor styling never reaches the OS — so we forward
 * Chromium's chosen cursor type to aboveView and let aboveView drive
 * `document.body.cursor`.
 *
 * Reconciles per layout pass: compares the predicate-derived keyboard
 * target against the currently-attached page and (de)attaches the
 * `cursor-changed` listener accordingly. Called from `layoutAllViews()`
 * alongside `reconcileFocus()`.
 */

import { ipcChannels } from '../../shared/ipc-contract'
import { findPageById } from './runtime-context'
import { currentKeyboardTargetPageId } from './selection-controller'
import { aboveView } from './view-refs'
import { safeSend } from './safe-send'

type CursorChangeEvent = (event: Electron.Event, type: string) => void

let attachedPageId: string | null = null
let attachedListener: CursorChangeEvent | null = null

function detach(): void {
  if (!attachedPageId || !attachedListener) {
    attachedPageId = null
    attachedListener = null
    return
  }
  const page = findPageById(attachedPageId)
  if (page && !page.pageView.webContents.isDestroyed()) {
    page.pageView.webContents.off('cursor-changed', attachedListener)
  }
  attachedPageId = null
  attachedListener = null
}

function attach(pageId: string): void {
  const page = findPageById(pageId)
  if (!page || page.pageView.webContents.isDestroyed()) return
  const listener: CursorChangeEvent = (_event, type) => {
    if (!aboveView || aboveView.webContents.isDestroyed()) return
    safeSend(aboveView.webContents, ipcChannels.aboveviewCursorUpdate, { type })
  }
  page.pageView.webContents.on('cursor-changed', listener)
  attachedPageId = pageId
  attachedListener = listener
}

function reset(): void {
  if (!aboveView || aboveView.webContents.isDestroyed()) return
  safeSend(aboveView.webContents, ipcChannels.aboveviewCursorUpdate, { type: null })
}

export function reconcilePageCursorBridge(): void {
  const target = currentKeyboardTargetPageId()
  if (target === attachedPageId) return
  detach()
  if (target) {
    attach(target)
  } else {
    reset()
  }
}
