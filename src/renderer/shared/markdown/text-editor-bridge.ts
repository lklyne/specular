/**
 * Publishes whichever text-bearing entity (sticky note or .md file) is
 * currently being edited so its selection popup can render formatting
 * toggle state and dispatch commands without threading callbacks through
 * the selection popup table.
 *
 * Module-scope single slot: main owns `editingEntityId`, and only one
 * entity can be in inline-edit mode at a time, so one slot suffices — no
 * per-entity map. `useSyncExternalStore` means only the popover re-renders
 * on cursor moves, not the whole editing card.
 */

import { useCallback, useEffect, useRef, useSyncExternalStore } from 'react'
import type { EditorState, StateCommand } from '@codemirror/state'
import type { EditorView } from '@codemirror/view'
import { selectionFormatState, type StickyFormatState } from './markdown-format-state'

export interface ActiveTextEditor {
  entityId: string
  format: StickyFormatState
  exec: (command: StateCommand) => void
}

let current: ActiveTextEditor | null = null
const listeners = new Set<() => void>()

function publishActiveTextEditor(entry: ActiveTextEditor | null): void {
  current = entry
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

function getSnapshot(): ActiveTextEditor | null {
  return current
}

export function useActiveTextEditor(): ActiveTextEditor | null {
  return useSyncExternalStore(subscribe, getSnapshot)
}

/**
 * Publishes an editing entity's live EditorView + cursor formatting state so
 * its selection popup can render a formatting section and dispatch commands
 * into this exact editor. Used by sticky notes (StickyBodyLayer) and .md
 * files (MarkdownInlineRenderer) — only one entity edits at a time
 * (main-owned `editingEntityId`), so the single slot is always this hook's
 * own entity while `enabled` is true.
 */
export function useEditorBridge(
  entityId: string,
  enabled: boolean,
): {
  onViewReady: (view: EditorView | null) => void
  onSelectionChange: (state: EditorState) => void
} {
  const viewRef = useRef<EditorView | null>(null)
  const enabledRef = useRef(enabled)
  enabledRef.current = enabled
  const exec = useCallback((command: StateCommand) => {
    const view = viewRef.current
    if (!view) return
    command({ state: view.state, dispatch: view.dispatch })
    view.focus()
  }, [])
  const onViewReady = useCallback((view: EditorView | null) => {
    viewRef.current = view
  }, [])
  const onSelectionChange = useCallback(
    (state: EditorState) => {
      if (!enabledRef.current) return
      publishActiveTextEditor({ entityId, format: selectionFormatState(state), exec })
    },
    [entityId, exec],
  )

  // The editor outlives edit mode (one view, reconfigured — see
  // MarkdownEditor), so the slot is claimed and released on the `enabled`
  // edge rather than on mount/unmount.
  useEffect(() => {
    const view = viewRef.current
    if (!enabled || !view) return
    publishActiveTextEditor({ entityId, format: selectionFormatState(view.state), exec })
    return () => publishActiveTextEditor(null)
  }, [enabled, entityId, exec])

  return { onViewReady, onSelectionChange }
}
