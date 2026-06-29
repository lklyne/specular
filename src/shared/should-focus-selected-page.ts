/**
 * Returns the page id that should hold keyboard input and receive forwarded
 * pointer/wheel events, or null when canvas-level UI (aboveView, an inline
 * editor, a tool gesture) owns keyboard instead.
 *
 * Selection — not a separate "focus" state — drives forwarding. The four
 * divergence cases from the plan are baked in:
 *
 *   1. Inline entity editor active over a single-selected page
 *      (`interactionKind === 'editing-entity'`) — keystrokes go to the
 *      editor's contenteditable, not the page.
 *   2. `activeTool.kind === 'draw'` with a page selected — strokes are
 *      canvas-bound; the page must not capture keys.
 *   3. Active drag of a single-selected page
 *      (`interactionKind === 'dragging-entities'`) — Escape and arrow
 *      handling stay with the canvas.
 *   4. `activeTool.kind === 'inspect'` or `'comment'` with a page
 *      selected — keyboard goes to the canvas (Escape exits the tool).
 *      When the comment composer opens, `commentOverlayActive` flips and
 *      keeps the predicate at null.
 */

import type { CanvasEntityKind, Tool } from './types'
import type { InteractionMode } from './interaction-types'

export type FocusSelectionInput =
  | { kind: 'none' }
  | { kind: 'single-entity'; entityId: string; entityKind: CanvasEntityKind }
  | { kind: 'multi-entity'; entityIds: readonly string[] }

export type ShouldFocusSelectedPageInputs = {
  selection: FocusSelectionInput
  interactionKind: InteractionMode['kind']
  activeTool: Tool
  commentOverlayActive: boolean
  /** The page the user has *entered* for interaction (#124). A merely-selected
   *  page does not own keyboard — only the entered page does. */
  interactivePageId: string | null
}

export function shouldFocusSelectedPage(
  inputs: ShouldFocusSelectedPageInputs,
): string | null {
  if (inputs.selection.kind !== 'single-entity') return null
  if (inputs.selection.entityKind !== 'page') return null
  if (inputs.interactionKind !== 'idle') return null
  if (inputs.activeTool.kind !== 'select') return null
  if (inputs.commentOverlayActive) return null
  // Select-first / interact-second: a selected page owns keyboard only once
  // the user has entered it (second click / double-click).
  if (inputs.selection.entityId !== inputs.interactivePageId) return null
  return inputs.selection.entityId
}
