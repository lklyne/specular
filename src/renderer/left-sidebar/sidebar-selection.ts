import type { SelectionMutationMode } from '../../shared/selection-modifiers'

export type SidebarSelectionIntent = {
  ids: string[]
  mode: SelectionMutationMode
  nextAnchorId: string
}

export function sidebarSelectionIntent(input: {
  clickedId: string
  orderedVisibleIds: readonly string[]
  lastClickedId: string | null
  shiftKey: boolean
  toggleKey: boolean
}): SidebarSelectionIntent {
  const { clickedId, orderedVisibleIds, lastClickedId, shiftKey, toggleKey } = input

  if (shiftKey && lastClickedId) {
    const anchorIndex = orderedVisibleIds.indexOf(lastClickedId)
    const clickedIndex = orderedVisibleIds.indexOf(clickedId)
    if (anchorIndex !== -1 && clickedIndex !== -1) {
      const start = Math.min(anchorIndex, clickedIndex)
      const end = Math.max(anchorIndex, clickedIndex)
      return {
        ids: [...new Set(orderedVisibleIds.slice(start, end + 1))],
        mode: 'add',
        nextAnchorId: clickedId,
      }
    }
  }

  return {
    ids: [clickedId],
    mode: toggleKey ? 'toggle' : 'replace',
    nextAnchorId: clickedId,
  }
}
