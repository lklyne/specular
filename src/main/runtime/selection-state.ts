import {
  pages,
  setHoverTarget,
} from './runtime-context'
import { activeWorkspaceTabId, workspaceTabs } from './workspace-model'
import {
  activeTool as uiActiveTool,
  devtoolsPanelTab as uiDevtoolsPanelTab,
  selectedEntityIds as uiSelectedEntityIds,
  selectedPageIndex as uiSelectedPageIndex,
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

type ArrowDirection = 'left' | 'right' | 'up' | 'down'

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

export function selectAdjacentPage(direction: ArrowDirection): boolean {
  if (!pages.length) return false
  const pageOrder = workspaceTabs
    .find((tab) => tab.id === activeWorkspaceTabId)
    ?.snapshot.pages.map((page) => page.id)
    .filter((id): id is string => Boolean(id))
  if (!pageOrder?.length) return false
  const selectedIdx = uiSelectedPageIndex(pages.map((p) => p.id))
  const currentSelectedPageId =
    selectedIdx !== null && selectedIdx >= 0 && selectedIdx < pages.length
      ? pages[selectedIdx].id
      : null
  const currentPageId =
    currentSelectedPageId ?? uiSelectedEntityIds()[0] ?? pageOrder[0]
  const currentOrderIndex = pageOrder.indexOf(currentPageId)
  const baseOrderIndex = currentOrderIndex >= 0 ? currentOrderIndex : 0
  const step = direction === 'left' || direction === 'up' ? -1 : 1
  const nextOrderIndex = (baseOrderIndex + step + pageOrder.length) % pageOrder.length
  const nextPageId = pageOrder[nextOrderIndex]
  return selectPageById(nextPageId)
}
