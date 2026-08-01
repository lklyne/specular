import { useCallback, useRef } from 'react'
import type { EditorState, StateCommand } from '@codemirror/state'
import type { EditorView } from '@codemirror/view'
import { selectionFormatState } from '../shared/markdown/markdown-format-state'
import { publishActiveTextEditor } from './textEditorBridge'

/**
 * Publishes an editing entity's live EditorView + cursor formatting state to
 * `textEditorBridge` so its selection popup can render a formatting section
 * and dispatch commands into this exact editor. Shared by sticky notes
 * (StickyBodyLayer) and .md files (FileBodyLayer) — only one entity edits
 * at a time (main-owned `editingEntityId`), so the bridge's single slot is
 * always this hook's own entity while `enabled` is true.
 */
export function useEditorBridge(
  entityId: string,
  enabled: boolean,
): {
  onViewReady?: (view: EditorView | null) => void
  onSelectionChange?: (state: EditorState) => void
} {
  const viewRef = useRef<EditorView | null>(null)
  const exec = useCallback((command: StateCommand) => {
    const view = viewRef.current
    if (!view) return
    command({ state: view.state, dispatch: view.dispatch })
    view.focus()
  }, [])
  const onViewReady = useCallback(
    (view: EditorView | null) => {
      viewRef.current = view
      if (!view) {
        publishActiveTextEditor(null)
        return
      }
      publishActiveTextEditor({ entityId, format: selectionFormatState(view.state), exec })
    },
    [entityId, exec],
  )
  const onSelectionChange = useCallback(
    (state: EditorState) => {
      publishActiveTextEditor({ entityId, format: selectionFormatState(state), exec })
    },
    [entityId, exec],
  )
  if (!enabled) return {}
  return { onViewReady, onSelectionChange }
}
