/**
 * FocusReconciler — expected-focus model for the interaction layer.
 *
 * Spec §4.4. Rather than chasing focus reactively (which triggers
 * focus storms, stale focus on macOS — see gotchas #5, #6), we declare
 * which webContents SHOULD own focus given current state, and reconcile
 * actual vs expected once per layout pass.
 *
 * `expectedFocus(state)` is pure. `reconcile()` compares the actual
 * focused webContents to the expected target and calls `.focus()` at
 * most once per pass — only on mismatch. It MUST run after the layout
 * pass has applied bounds/visibility, so focus lands on a view that's
 * actually visible.
 *
 * Phase 3 scope: reconciler is additive. Existing imperative `focus()`
 * call sites keep working; the reconciler corrects any drift they
 * miss (e.g. the `did-finish-load` focus-steal, gotcha #5). Migration
 * of imperative calls to focus-intent + markDirty happens in Phase 5.
 */

import type { FocusTarget } from '../../shared/interaction-types'

export type FocusState = {
  interactionMode: 'idle' | 'panning' | 'marquee' | 'dragging-entities' | 'resizing-entity' | 'resizing-multi-selection' | 'dragging-edge' | 'editing-entity' | 'reordering-row'
  editingEntityId: string | null
  selectedPageId: string | null
  workspaceViewMode: 'canvas' | 'browser'
  commentOverlayActive: boolean
  /** Explicit intent set by a subsystem (overrides derivation). Cleared after reconcile. */
  pendingFocus: FocusTarget | null
  /** Predicate-derived: the page id that should hold keyboard + receive
   *  forwarded input, or null. Filled from `currentKeyboardTargetPageId`
   *  at the runtime binding layer. */
  focusedPageId: string | null
  /** True while a sidebar rename input is active. Forces sidebar focus so
   *  layout passes don't steal focus back to the canvas. */
  sidebarTextInputActive: boolean
}

export function expectedFocus(state: FocusState): FocusTarget {
  if (state.pendingFocus) return state.pendingFocus

  // Sidebar rename input is open — keep focus in the sidebar so keystrokes
  // reach the input rather than firing canvas shortcuts.
  if (state.sidebarTextInputActive) return { kind: 'sidebar' }

  // Selection-driven page focus (the predicate already gates on idle
  // interaction + commentOverlayActive). Gesture modes still win below
  // — a drag started on canvas chrome should not be hijacked.
  if (
    state.focusedPageId &&
    state.interactionMode === 'idle' &&
    !state.commentOverlayActive
  ) {
    return { kind: 'page', id: state.focusedPageId }
  }

  // Gesture-active: input gate (aboveView) owns focus.
  switch (state.interactionMode) {
    case 'panning':
    case 'marquee':
    case 'dragging-entities':
    case 'resizing-entity':
    case 'resizing-multi-selection':
    case 'dragging-edge':
    case 'reordering-row':
      return { kind: 'aboveView' }
    case 'editing-entity':
      // Inline canvas editors (sticky notes, shapes, markdown files,
      // wireframes, group rename) render in aboveView, so keyboard focus
      // lives there while typing.
      return { kind: 'aboveView' }
    case 'idle':
      break
  }

  // Comment overlay active — the above-view owns focus so the
  // composer textarea can receive keyboard input.
  if (state.commentOverlayActive) {
    return { kind: 'aboveView' }
  }

  // Browser mode with a live page — the page should be focused so
  // keyboard input reaches the web content.
  if (state.workspaceViewMode === 'browser' && state.selectedPageId) {
    return { kind: 'page', id: state.selectedPageId }
  }

  // Canvas mode default: aboveView is the singleton keyboard owner. Canvas
  // shortcuts (Cmd-Z, Escape, tool hotkeys) are wired into aboveView's
  // webContents via `watchModifierKeys`. The browser-mode no-selected-page
  // fallback also lands here — a degenerate state with no page yet.
  return { kind: 'aboveView' }
}

/** Stable key for comparing FocusTargets without importing Electron refs. */
export function focusKey(t: FocusTarget): string {
  switch (t.kind) {
    case 'page': return `page:${t.id}`
    default: return t.kind
  }
}
