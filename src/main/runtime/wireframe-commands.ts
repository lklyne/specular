/**
 * Wireframe content commands (plan 3.0b) — the apply seam.
 *
 * Each command applies a wireframe edit through the runtime mirror, projects
 * the result to disk immediately (so the renderer's disk re-fetch stays fresh
 * while the read path lives on disk — 3.5b owns the repoint), and schedules
 * autosave. The autosave's forward sync writes the change to the Y.Doc
 * `wireframes` map as a single `'user'` transaction, which the UndoManager
 * captures — so wireframe edits undo/redo like any other workspace mutation.
 *
 * Apply path: runtime op → one Y.Doc transaction → reverse sync → projection to
 * disk → renderer re-fetch. The pure ops from 3.0 are the transaction bodies.
 */

import { scheduleWorkspaceAutosave } from './workspace-session'
import {
  applyWireframeOp,
  ensureWireframeBaseline,
  getWireframeContent,
  projectWireframeEntityToDisk,
  setWireframeContent,
  type WireframeOp,
} from './wireframe-content-state'
import { parseWireframeFile } from '../../shared/wireframe/wireframe-codec'
import { createNodeIdGenerator } from '../../shared/wireframe/wireframe-ops'
import {
  createWireframeNode,
  type WireframePaletteType,
} from '../../shared/wireframe/wireframe-node-factory'

/**
 * Apply a structural op (insert / delete / duplicate / reorder / setProps /
 * setText / toggle) to the named wireframe entity. Returns the new content, or
 * an error for an unknown entity / illegal op.
 */
export function commitWireframeOp(
  entityId: string,
  op: WireframeOp,
): { ok: true; content: string } | { ok: false; error: string } {
  ensureWireframeBaseline(entityId)
  const result = applyWireframeOp(entityId, op)
  if (!result.ok) return result
  projectWireframeEntityToDisk(entityId)
  scheduleWorkspaceAutosave()
  return result
}

/**
 * Replace a wireframe entity's content verbatim — the severed renderer write
 * path. The renderer already computed the full JSON; route it up as a `replace`
 * op rather than writing the file directly.
 */
export function commitWireframeContent(entityId: string, content: string): void {
  ensureWireframeBaseline(entityId)
  setWireframeContent(entityId, content)
  projectWireframeEntityToDisk(entityId)
  scheduleWorkspaceAutosave()
}

let _insertCounter = 0

/**
 * Insert a default node of `nodeType` (3.2 — the insert palette) into the
 * entity's root frame, at the end. Builds the node with the shared factory and
 * applies it through `commitWireframeOp`, so it is one undoable Y.Doc op. The
 * panel has no view of the tree, so the parent is resolved here (the root).
 */
export function commitWireframeInsertNode(
  entityId: string,
  nodeType: WireframePaletteType,
): { ok: true; content: string } | { ok: false; error: string } {
  ensureWireframeBaseline(entityId)
  const current = getWireframeContent(entityId)
  if (current == null) return { ok: false, error: `Unknown wireframe entity: ${entityId}` }

  let parentId: string
  let index: number
  try {
    const file = parseWireframeFile(current)
    parentId = file.root.id
    index = file.root.type === 'frame' ? file.root.children.length : 0
  } catch (err) {
    return { ok: false, error: (err as Error).message }
  }

  _insertCounter += 1
  const node = createWireframeNode(nodeType, createNodeIdGenerator(`add${_insertCounter}`))
  return commitWireframeOp(entityId, { kind: 'insert', parentId, index, node })
}
