import type {
  CanvasSelectableTarget,
  WorkspaceEdge,
} from '../../shared/types'

/**
 * EdgePopup currently edits one edge at a time. A selected edge is its popup
 * subject only when it is also the entire selection; mixed selections belong
 * to the multi-select popup.
 */
export function edgeForPopup(
  selection: readonly CanvasSelectableTarget[],
  edges: readonly WorkspaceEdge[],
): WorkspaceEdge | null {
  if (selection.length !== 1 || selection[0].kind !== 'edge') return null
  return edges.find((edge) => edge.id === selection[0].id) ?? null
}
