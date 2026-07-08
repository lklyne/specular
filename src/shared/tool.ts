// Unified Tool concept — see ADR 0005, amended by ADR 0006 (comment tool).
// Per ADR 0009, `add-shape` and `draw` no longer carry sub-kind variants;
// those move to tool defaults (ADR 0008 §9) and are surfaced through the
// tool-mode popup.

export type DrawingBrushType = 'pen' | 'highlight'

export type Tool =
  | { kind: 'select' }
  | { kind: 'hand' }
  | { kind: 'add-page'; presetIndex?: number; customSize?: boolean; sourcePageId?: string }
  | { kind: 'add-text' }
  | { kind: 'add-document' }
  | { kind: 'add-sticky' }
  | { kind: 'add-shape' }
  | { kind: 'comment' }
  | { kind: 'draw' }
  | { kind: 'inspect' }

export type ToolKind = Tool['kind']

export type ToolDuration = 'one-shot' | 'persistent'

export const toolDuration: Record<ToolKind, ToolDuration> = {
  select: 'persistent',
  hand: 'persistent',
  'add-page': 'one-shot',
  'add-text': 'one-shot',
  'add-document': 'one-shot',
  'add-sticky': 'one-shot',
  'add-shape': 'one-shot',
  comment: 'persistent',
  draw: 'persistent',
  inspect: 'persistent',
}

export function isOneShot(kind: ToolKind): boolean {
  return toolDuration[kind] === 'one-shot'
}

export function isPersistent(kind: ToolKind): boolean {
  return toolDuration[kind] === 'persistent'
}

/**
 * Tools that own a viewport-anchored tool-mode popup (ADR 0008 §1, §2). When
 * any of these is active, selection-driven popups are suppressed (mutex rule
 * §2) so the user sees one popup at a time — the tool's, not the previous
 * selection's. `add-page`, `comment`, `inspect`, and `select`
 * have no popup and don't suppress anything.
 */
export function toolHasPopup(tool: Tool): boolean {
  return (
    tool.kind === 'add-page' ||
    tool.kind === 'add-text' ||
    tool.kind === 'add-sticky' ||
    tool.kind === 'add-shape' ||
    tool.kind === 'draw'
  )
}

export function isAnnotationTool(tool: Tool): boolean {
  return tool.kind === 'comment' || tool.kind === 'draw'
}

export function isPlacementTool(tool: Tool): boolean {
  return (
    tool.kind === 'add-page' ||
    tool.kind === 'add-text' ||
    tool.kind === 'add-document' ||
    tool.kind === 'add-sticky' ||
    tool.kind === 'add-shape'
  )
}

/**
 * Working tools express creation/annotation intent — placing items, drawing, or
 * commenting. The resting tools (select, hand, inspect) do not. Used as the one
 * definition of "am I working" for focus: a working tool keeps a focus session
 * alive across camera changes (ADR 0021) and lifts the focus dim.
 */
export function isWorkingTool(tool: Tool): boolean {
  return isPlacementTool(tool) || isAnnotationTool(tool)
}

export function applyPlacementCompletion(current: Tool): Tool {
  return isOneShot(current.kind) ? SELECT_TOOL : current
}

export function applyEscape(_current: Tool): Tool {
  return SELECT_TOOL
}

export const SELECT_TOOL: Tool = { kind: 'select' }

// Page-content overlay vocabulary (legacy IPC). Kept narrow on purpose —
// renderer-side mode that drives the comment-hover affordance, intentionally
// not part of the unified Tool vocabulary.
export type AnnotateOverlayMode = 'off' | 'comment' | 'draw'

export function toolAnnotateOverlay(tool: Tool): {
  enabled: boolean
  mode: AnnotateOverlayMode
} {
  // ADR 0006: comment tool's element/region preview is now painted by the
  // page itself in response to `comment-tool-pointer-state` broadcasts from
  // main; the legacy in-page hover affordance is retired. The overlay mode
  // remains exposed only for `draw` (which still relies on the page-side
  // overlay to suppress native input) and as an `off` no-op default.
  if (tool.kind === 'draw') return { enabled: false, mode: 'draw' }
  return { enabled: false, mode: 'off' }
}

// Lowercase gerund for cursor labels, status-bar narration, and live captions.
export function toolGerund(tool: Tool): string {
  switch (tool.kind) {
    case 'select':
      return 'selecting'
    case 'hand':
      return 'panning'
    case 'add-page':
      return 'adding page'
    case 'add-text':
      return 'adding text'
    case 'add-document':
      return 'adding document'
    case 'add-sticky':
      return 'adding sticky note'
    case 'add-shape':
      return 'adding shape'
    case 'comment':
      return 'commenting'
    case 'draw':
      return 'drawing'
    case 'inspect':
      return 'inspecting'
  }
}
