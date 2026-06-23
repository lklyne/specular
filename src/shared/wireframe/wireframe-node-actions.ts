// Selected-node action controller for the wireframe canvas editor (plan 3.2).
//
// A thin pure layer over the structural ops (3.0): it maps a high-level canvas
// action (delete / duplicate the *selected* node) plus the current tree to the
// next file and the node that should be selected afterwards. The gesture wiring
// in WireframeRenderer stays a shell over this testable logic — the same shape
// the selection reducer follows.

import type { WireframeFile } from './wireframe-types'
import { createNodeIdGenerator, deleteNode, duplicateNode, findNodeById } from './wireframe-ops'

export type WireframeNodeAction = 'delete' | 'duplicate'

export interface WireframeNodeActionResult {
  /** The mutated file (a new object — never the input). */
  file: WireframeFile
  /**
   * The node to select after the action: `null` after delete (selection clears),
   * the clone's id after duplicate (so the new copy is selected for further edits).
   */
  nextSelectedNodeId: string | null
}

/**
 * Apply a selected-node action. Returns `null` (a no-op the caller skips) when
 * nothing is selected, the id is unknown, or the action targets the root frame —
 * the root can be neither deleted (a wireframe always has a root) nor duplicated
 * (it has no parent to duplicate into).
 *
 * `seq` makes duplicate's fresh ids deterministic-by-input: pass a per-invocation
 * counter so repeated duplicates don't collide. Ignored for delete.
 */
export function applyNodeAction(
  file: WireframeFile,
  action: WireframeNodeAction,
  selectedNodeId: string | null,
  seq: number,
): WireframeNodeActionResult | null {
  if (!selectedNodeId) return null
  if (selectedNodeId === file.root.id) return null
  if (!findNodeById(file.root, selectedNodeId)) return null

  if (action === 'delete') {
    const next = deleteNode(file, selectedNodeId)
    if (next === file) return null
    return { file: next, nextSelectedNodeId: null }
  }

  // duplicate — capture the first generated id (the clone's root) so we can
  // select the new copy regardless of how the clone walks the subtree.
  let cloneRootId: string | null = null
  const base = createNodeIdGenerator(`dup${seq}`)
  const genId = () => {
    const id = base()
    if (cloneRootId === null) cloneRootId = id
    return id
  }
  const next = duplicateNode(file, selectedNodeId, genId)
  if (next === file) return null
  return { file: next, nextSelectedNodeId: cloneRootId }
}
