// Pure selection controller for the wireframe canvas editor.
//
// Selection is *ephemeral* runtime state — it lives only in the renderer and is
// never written to the Y.Doc (unlike node content). The reducer below maps a
// pointer/keyboard intent plus the current tree to the next selection state, so
// the gesture wiring in WireframeRenderer stays a thin shell over testable logic.

import type { WireframeFile, WireframeNode } from './wireframe-types'
import { findNodeById } from './wireframe-ops'

export interface WireframeSelectionState {
  /** The node drawn with a selection outline, or null when nothing is selected. */
  selectedNodeId: string | null
  /** The node whose text is being edited inline, or null when not editing. */
  editingNodeId: string | null
}

export const EMPTY_WIREFRAME_SELECTION: WireframeSelectionState = {
  selectedNodeId: null,
  editingNodeId: null,
}

export type WireframeSelectionIntent =
  /** Single click on a node — select it (and drop out of any active edit). */
  | { kind: 'select-node'; nodeId: string }
  /** Click on empty canvas — clear selection and edit. */
  | { kind: 'select-background' }
  /** Double-click / Enter — promote a node to inline text edit when editable. */
  | { kind: 'request-edit'; nodeId: string }
  /** Inline edit finished — keep the node selected. */
  | { kind: 'commit-edit' }
  /** Esc — step out of edit first, then clear selection. */
  | { kind: 'escape' }

/** Node types whose text content can be edited inline on the canvas. */
export function nodeHasEditableText(node: WireframeNode): boolean {
  return (
    node.type === 'text' ||
    node.type === 'button' ||
    node.type === 'input' ||
    node.type === 'dropdown' ||
    node.type === 'checkbox' ||
    node.type === 'toggle'
  )
}

function selectOnly(
  state: WireframeSelectionState,
  nodeId: string,
): WireframeSelectionState {
  if (state.selectedNodeId === nodeId && state.editingNodeId === null) return state
  return { selectedNodeId: nodeId, editingNodeId: null }
}

export function wireframeSelectionReducer(
  state: WireframeSelectionState,
  intent: WireframeSelectionIntent,
  file: WireframeFile | null,
): WireframeSelectionState {
  switch (intent.kind) {
    case 'select-node':
      return selectOnly(state, intent.nodeId)

    case 'select-background':
      if (state.selectedNodeId === null && state.editingNodeId === null) return state
      return EMPTY_WIREFRAME_SELECTION

    case 'request-edit': {
      const node = file ? findNodeById(file.root, intent.nodeId) : null
      // Non-editable nodes (frame/image/divider/spacer) can still be selected,
      // they just can't enter text edit.
      if (!node || !nodeHasEditableText(node)) return selectOnly(state, intent.nodeId)
      if (state.editingNodeId === intent.nodeId) return state
      return { selectedNodeId: intent.nodeId, editingNodeId: intent.nodeId }
    }

    case 'commit-edit':
      if (state.editingNodeId === null) return state
      return { selectedNodeId: state.editingNodeId, editingNodeId: null }

    case 'escape':
      if (state.editingNodeId !== null) {
        return { selectedNodeId: state.editingNodeId, editingNodeId: null }
      }
      if (state.selectedNodeId === null) return state
      return EMPTY_WIREFRAME_SELECTION

    default:
      return state
  }
}
