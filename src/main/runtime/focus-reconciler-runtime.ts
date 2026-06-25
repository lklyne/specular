// fallow-ignore-file circular-dependencies
// Suppressed: see #141. selection-controller → overlay-manager → layout-engine import focus-reconciler-runtime back
/**
 * Runtime binding for FocusReconciler. Resolves a FocusTarget to the
 * actual WebContents and calls focus() at most once per layout pass.
 * Kept separate from focus-reconciler.ts so the pure expectedFocus()
 * function stays unit-testable without Electron.
 */

import type { WebContents } from 'electron'
import type { FocusTarget } from '../../shared/interaction-types'
import { canvasInteractionModeKind } from '../../shared/gesture-utils'
import { expectedFocus, focusKey, type FocusState } from './focus-reconciler'
import { aboveView, bgView, toolbarView, leftSidebarView, win } from './view-refs'
import {
  getEditingEntityId,
  pages,
  interactionState,
  pendingFocus,
  setPendingFocus,
} from './runtime-context'
import { isTextEditingFor } from './binding-dispatcher'
import { isCommentOverlayVisible, selectedPageIndex } from '../ui-state'
import { currentKeyboardTargetPageId } from './selection-controller'

function currentFocusState(): FocusState {
  const idx = selectedPageIndex(pages.map((p) => p.id))
  const selectedPage = idx != null ? pages[idx] : null
  return {
    interactionMode: canvasInteractionModeKind(interactionState),
    editingEntityId: getEditingEntityId(),
    selectedPageId: selectedPage?.id ?? null,
    commentOverlayActive: isCommentOverlayVisible(),
    pendingFocus,
    focusedPageId: currentKeyboardTargetPageId(),
    sidebarTextInputActive: leftSidebarView ? isTextEditingFor(leftSidebarView.webContents) : false,
  }
}

function resolve(target: FocusTarget): WebContents | null {
  switch (target.kind) {
    case 'bgView': return bgView?.webContents ?? null
    case 'aboveView': return aboveView?.webContents ?? null
    case 'toolbar': return toolbarView?.webContents ?? null
    case 'sidebar': return leftSidebarView?.webContents ?? null
    case 'page': {
      const page = pages.find((p) => p.id === target.id)
      return page?.pageView.webContents ?? null
    }
  }
}

function currentlyFocusedKey(): string | null {
  if (bgView?.webContents.isFocused()) return 'bgView'
  if (aboveView?.webContents.isFocused()) return 'aboveView'
  if (toolbarView?.webContents.isFocused()) return 'toolbar'
  if (leftSidebarView?.webContents.isFocused()) return 'sidebar'
  for (const p of pages) {
    if (p.pageView.webContents.isFocused()) return `page:${p.id}`
  }
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
