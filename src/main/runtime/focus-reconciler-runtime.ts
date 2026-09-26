/**
 * Runtime binding for FocusReconciler. Resolves a FocusTarget to the
 * actual WebContents and calls focus() at most once per layout pass.
 * Kept separate from focus-reconciler.ts so the pure expectedFocus()
 * function stays unit-testable without Electron.
 *
 * A `{ kind: 'page' }` target resolves to aboveView, not to the page. Pages
 * render offscreen, where `focus()` is a no-op and OS key events go to the
 * focused native view — so aboveView holds OS keyboard focus always, and "the
 * page has focus" is expressed by `page-focus-emulation.ts` plus the key
 * forwarding aboveView's sink drives.
 */

import type { WebContents } from 'electron'
import type { FocusTarget } from '../../shared/interaction-types'
import { canvasInteractionModeKind } from '../../shared/gesture-utils'
import { expectedFocus, focusKey, type FocusState } from './focus-reconciler'
import { aboveView, bgView, toolbarView, leftSidebarView, win } from './view-refs'
import {
  getEditingEntityId,
  interactionState,
  pendingFocus,
  setPendingFocus,
} from './runtime-context'
import { isTextEditingFor } from './binding-dispatcher'
import { isCommentOverlayVisible, toolbarDropdownOpen } from '../ui-state'
import { currentKeyboardTargetPageId } from './selection-controller'

function currentFocusState(): FocusState {
  return {
    interactionMode: canvasInteractionModeKind(interactionState),
    editingEntityId: getEditingEntityId(),
    commentOverlayActive: isCommentOverlayVisible(),
    pendingFocus,
    focusedPageId: currentKeyboardTargetPageId(),
    sidebarTextInputActive: leftSidebarView ? isTextEditingFor(leftSidebarView.webContents) : false,
    toolbarTextInputActive: toolbarView ? isTextEditingFor(toolbarView.webContents) : false,
    toolbarDropdownOpen: toolbarDropdownOpen(),
  }
}

function resolve(target: FocusTarget): WebContents | null {
  switch (target.kind) {
    case 'bgView': return bgView?.webContents ?? null
    case 'aboveView': return aboveView?.webContents ?? null
    case 'toolbar': return toolbarView?.webContents ?? null
    case 'sidebar': return leftSidebarView?.webContents ?? null
    case 'page': return aboveView?.webContents ?? null
  }
}

export function currentlyFocusedKey(): string | null {
  if (bgView?.webContents.isFocused()) return 'bgView'
  if (aboveView?.webContents.isFocused()) {
    // aboveView owns OS focus on the keyboard-target page's behalf, so when
    // there is one it is the page that is focused as far as this ladder is
    // concerned — otherwise the reconciler would call focus() every pass.
    const pageId = currentKeyboardTargetPageId()
    return pageId ? `page:${pageId}` : 'aboveView'
  }
  if (toolbarView?.webContents.isFocused()) return 'toolbar'
  if (leftSidebarView?.webContents.isFocused()) return 'sidebar'
  return null
}

/**
 * Compare actual focus to expected. Call focus() at most once.
 * Call this at the end of layoutAllViews(), after bounds/visibility
 * mutations — otherwise focus may land on a 0-size view.
 *
 * Runs unconditionally on every layout pass (Phase 5d-v2: D4). All six
 * former imperative focus() callers set `pendingFocus` + request a
 * layout; this reconciler is the single site that actually calls
 * webContents.focus() in the main process.
 */
export function reconcileFocus(): void {
  if (!win || win.isDestroyed()) return
  if (!win.isFocused()) return

  const state = currentFocusState()
  const expected = expectedFocus(state)
  if (focusKey(expected) !== currentlyFocusedKey()) {
    const target = resolve(expected)
    if (target && !target.isDestroyed()) target.focus()
  }
  if (pendingFocus) setPendingFocus(null)
}
