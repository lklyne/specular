import {
  pages,
  setHoverTarget,
} from './runtime-context'
import {
  activeTool as uiActiveTool,
  devtoolsPanelTab as uiDevtoolsPanelTab,
  setActiveTool as setUiActiveTool,
  setDevtoolsPanelTab as setUiDevtoolsPanelTab,
} from '../ui-state'
import {
  selectPages,
  selectNone,
  selectPageById,
} from './selection-controller'
import { cancelActive as cancelActiveInteraction } from './interaction-controller'
import { requestLayout } from './viewport-control'

export function selectPage(index: number): void {
  void selectPageByIndex(index)
}

export function selectPageByIndex(index: number): boolean {
  if (index < 0 || index >= pages.length) return false
  return selectPageById(pages[index].id)
}

export function setSelectedPages(pageIds: string[]): void {
  void selectPages(pageIds)
}

export function deselectAll(): void {
  void selectNone()
}

function clearTransientSelectionState(): void {
  cancelActiveInteraction('external')
  setHoverTarget(null)
  if (uiActiveTool().kind !== 'select') {
    setUiActiveTool({ kind: 'select' })
  }
}

export function normalizeCanvasSelectionState(): void {
  clearTransientSelectionState()
  if (uiDevtoolsPanelTab() === 'browser-devtools') {
    setUiDevtoolsPanelTab('comments')
  }
  requestLayout()
}
