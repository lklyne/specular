/**
 * Gate predicate — should the aboveView overlay cover the canvas?
 *
 * In canvas mode the gate is unconditionally open: aboveView is the
 * interactive layer. Pointer/wheel events that hit the single-selected
 * page's body are forwarded into the page from inside aboveView; chrome,
 * selection outlines, marquee, drawings, and overlays keep painting and
 * intercepting input there.
 *
 * Pure and testable. Authority lives in main.
 */
import type { Tool } from '../../shared/types'

export type GateInputs = {
  activeTool: Tool
  /** Imperative override set by IPC handlers that open annotation/comment UI. */
  commentOverlayActive: boolean
}

export function shouldGateBeOpen(inputs: GateInputs): boolean {
  const toolKind = inputs.activeTool.kind
  // Inspect drives feedback off the page's webContents mousemove
  // (eyedropper). Keep the gate closed unless the comment composer has been
  // opened by a different UI path.
  if (toolKind === 'inspect') {
    return inputs.commentOverlayActive
  }
  return true
}
