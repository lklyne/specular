/**
 * Overlay and interaction management — canvas interaction mode, selection marquee.
 */

import { ipcChannels } from '../../shared/ipc-contract'
import type { SelectionOverlayPayload } from '../../shared/types'
import {
  aboveView,
  bgView,
  win,
} from './view-refs'
import { layoutCache } from './layout-cache'
import { setBoundsIfChanged } from './layout-engine'
import {
  automationInteractivePageCounts,
  interactivePageId,
  pages,
  removeAutomationInteractivePageId,
  addAutomationInteractivePageId,
  setInteractivePageId,
  setSelectionOverlayActive,
} from './runtime-context'
import { workspaceGroups } from './workspace-model'
import {
  getUiState,
  isSelectionMarqueeVisible as uiSelectionMarqueeVisible,
  setSelectionMarqueeVisible as setUiSelectionMarqueeVisible,
} from '../ui-state'
import { selectionDebug } from './runtime-constants'
import { requestLayout } from './viewport-control'
import { safeSend } from './safe-send'

export function pageSelectionOverlayStates(): Array<{
  pageId: string
  interactive: boolean
  multiSelected: boolean
}> {
  const ui = getUiState()
  const multiSelectedPageIds = new Set<string>()
  // Select-first / interact-second (#124): a page is interactive only once the
  // user has *entered* it. A merely single-selected page stays blocked.
  const enteredPageId = interactivePageId()

  if (ui.selection.kind === 'multi-entity') {
    for (const entityId of ui.selection.entityIds) {
      if (pages.some((page) => page.id === entityId)) {
        multiSelectedPageIds.add(entityId)
      }
    }
  } else if (ui.selection.kind === 'single-entity' && ui.selection.entityKind === 'group') {
    const groupId = ui.selection.entityId
    for (const page of pages) {
      let currentParentId = page.parentGroupId
      while (currentParentId) {
        if (currentParentId === groupId) {
          multiSelectedPageIds.add(page.id)
          break
        }
        currentParentId = workspaceGroups.find((candidate) => candidate.id === currentParentId)?.parentGroupId
      }
    }
  }

  return pages.map((page) => ({
    pageId: page.id,
    interactive: enteredPageId === page.id || automationInteractivePageCounts.has(page.id),
    multiSelected:
      enteredPageId !== page.id &&
      !automationInteractivePageCounts.has(page.id) &&
      multiSelectedPageIds.has(page.id),
  }))
}

export function sendInteractiveState(): void {
  const states = pageSelectionOverlayStates()
  for (let i = 0; i < pages.length; i++) {
    const isSelected = states[i]?.interactive ?? false
    const isMultiSelected = states[i]?.multiSelected ?? false
    selectionDebug('sendInteractiveState', {
      pageId: pages[i].id,
      pageIndex: i,
      interactive: isSelected,
      multiSelected: isMultiSelected,
    })
    const wc = pages[i].pageView.webContents
    safeSend(wc, ipcChannels.setInteractive, isSelected)
    safeSend(wc, ipcChannels.setMultiSelected, isMultiSelected)
  }
}

/**
 * Enter interactive mode on a page (#124, the second deliberate click /
 * double-click). Sets it as the entered page so it forwards pointer input and
 * owns keyboard, then re-broadcasts blocker state and reconciles focus.
 */
export function enterPageInteractive(pageId: string): void {
  if (!pages.some((page) => page.id === pageId)) return
  if (interactivePageId() === pageId) return
  setInteractivePageId(pageId)
  sendInteractiveState()
  requestLayout()
}

/** Exit interactive mode back to selected-only (Escape, page delete). */
export function exitPageInteractive(): void {
  if (interactivePageId() === null) return
  setInteractivePageId(null)
  sendInteractiveState()
  requestLayout()
}

export function beginAutomationInteractivePage(pageId: string): void {
  addAutomationInteractivePageId(pageId)
  sendInteractiveState()
  // The layout pass parks automation-interactive pages off-screen at their
  // logical viewport size, so an agent always has a real viewport even when
  // the page isn't visible on the canvas.
  requestLayout()
}

export function endAutomationInteractivePage(pageId: string): void {
  if (!automationInteractivePageCounts.has(pageId)) return
  removeAutomationInteractivePageId(pageId)
  sendInteractiveState()
  // Invalidate bounds key so layoutAllViews restores viewport culling.
  const page = pages.find((p) => p.id === pageId)
  if (page) {
    page.lastPageBoundsKey = undefined
  }
}

export function setSelectionOverlayRect(
  overlay: SelectionOverlayPayload | null,
): void {
  setSelectionOverlayActive(overlay !== null)
  setUiSelectionMarqueeVisible(overlay !== null)

  if (!win || win.isDestroyed()) return

  if (aboveView) {
    safeSend(aboveView.webContents, ipcChannels.canvasSelectionOverlay, overlay)
  }
  // canvas-bg consumes the same payload to render per-entity marquee
  // preview outlines (`overlay.entityIds`).
  if (bgView) {
    safeSend(bgView.webContents, ipcChannels.canvasSelectionOverlay, overlay)
  }
  // The gate predicate reads selectionMarqueeVisible, so a rect change
  // can flip aboveView bounds on/off. Bounds + visibility are centralized
  // in layoutAllViews — schedule it.
  requestLayout()
}
