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
  projectWireframeEntityToDisk,
  setWireframeContent,
  type WireframeOp,
} from './wireframe-content-state'

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
