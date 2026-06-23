import { useCallback, useEffect, useRef, useState } from 'react'
import type { WireframeFile, WireframeThemeName, DropTarget } from '../../../shared/wireframe/wireframe-types'
import { wireframeThemes } from './wireframe-themes'
import { WireframeNodeRenderer } from './WireframeNodeRenderer'
import {
  findNodeById,
  reorderNode,
  updateNodeText,
  toggleNodeState,
} from '../../../shared/wireframe/wireframe-ops'
import {
  applyNodeAction,
  type WireframeNodeAction,
} from '../../../shared/wireframe/wireframe-node-actions'
import {
  EMPTY_WIREFRAME_SELECTION,
  wireframeSelectionReducer,
  type WireframeSelectionIntent,
  type WireframeSelectionState,
} from '../../../shared/wireframe/wireframe-selection'

export const WIREFRAME_THEME_OPTIONS: { name: WireframeThemeName; color: string }[] = [
  { name: 'light', color: '#ffffff' },
  { name: 'dark', color: '#18181b' },
  { name: 'blueprint', color: '#0f2744' },
]

/** The selected node, flattened to its own props (no children) for the panel. */
export type WireframeSelectedNode = { id: string; type: string } & Record<string, unknown>

export function WireframeRenderer({
  content,
  canEdit,
  jsonMode = false,
  onContentChange,
  onSelectionChange,
}: {
  content: string
  canEdit: boolean
  jsonMode?: boolean
  onContentChange: (json: string) => void
  onSelectionChange?: (node: WireframeSelectedNode | null) => void
}) {
  const [wireframe, setWireframe] = useState<WireframeFile | null>(() => {
    try {
      return JSON.parse(content)
    } catch {
      return null
    }
  })
  const [jsonText, setJsonText] = useState(content)
  const [jsonError, setJsonError] = useState<string | null>(null)

  // Drag state
  const [draggedNodeId, setDraggedNodeId] = useState<string | null>(null)
  const [dropTarget, setDropTarget] = useState<DropTarget | null>(null)

  // Selection + edit state (ephemeral — never written to the Y.Doc).
  const [selection, setSelection] = useState<WireframeSelectionState>(
    EMPTY_WIREFRAME_SELECTION,
  )
  const { selectedNodeId, editingNodeId } = selection

  const pendingRef = useRef<{
    nodeId: string
    parentId: string
    x: number
    y: number
  } | null>(null)
  const wireframeRef = useRef(wireframe)
  wireframeRef.current = wireframe

  // Per-invocation counter so repeated duplicates get non-colliding fresh ids.
  const dupSeqRef = useRef(0)

  const dispatchSelection = useCallback((intent: WireframeSelectionIntent) => {
    setSelection((prev) => wireframeSelectionReducer(prev, intent, wireframeRef.current))
  }, [])

  // Mirror the selected node (its own props, no children) to the panel so it can
  // render per-node property editors (3.3). Re-fires when the props change — e.g.
  // a panel edit round-trips back through `content` — so the editors stay live.
  // De-duped against the last payload to avoid redundant IPC on unrelated renders.
  const lastSelectionRef = useRef<string | null>(null)
  useEffect(() => {
    if (!onSelectionChange) return
    let payload: WireframeSelectedNode | null = null
    if (canEdit && selectedNodeId && wireframe?.root) {
      const node = findNodeById(wireframe.root, selectedNodeId)
      if (node) {
        const { children: _children, ...rest } = node as WireframeSelectedNode & {
          children?: unknown
        }
        payload = rest
      }
    }
    const serialized = payload ? JSON.stringify(payload) : null
    if (serialized === lastSelectionRef.current) return
    lastSelectionRef.current = serialized
    onSelectionChange(payload)
  }, [canEdit, selectedNodeId, wireframe, onSelectionChange])

  // Sync external content changes (skip initial mount — already parsed in useState initializer)
  const isFirstRender = useRef(true)
  useEffect(() => {
    if (isFirstRender.current) {
      isFirstRender.current = false
      return
    }
    try {
      const parsed = JSON.parse(content)
      setWireframe(parsed)
      setJsonText(content)
      setJsonError(null)
    } catch {
      // keep current state if parse fails
    }
  }, [content])

  // Clear selection/edit state when edit mode is lost
  useEffect(() => {
    if (!canEdit) {
      setSelection(EMPTY_WIREFRAME_SELECTION)
      setDraggedNodeId(null)
      setDropTarget(null)
    }
  }, [canEdit])

  // Enter promotes the selected node to edit; Esc steps out of edit, then clears.
  // The inline edit <input> handles its own keys (and stops propagation), so this
  // window listener only fires when not actively typing in a field.
  useEffect(() => {
    if (!canEdit) return
    if (!selectedNodeId && !editingNodeId) return
    const onKeyDown = (e: KeyboardEvent) => {
      const active = document.activeElement
      const tag = active?.tagName
      if (tag === 'INPUT' || tag === 'TEXTAREA') return
      if (e.key === 'Enter') {
        if (selectedNodeId && !editingNodeId) {
          e.preventDefault()
          dispatchSelection({ kind: 'request-edit', nodeId: selectedNodeId })
        }
      } else if (e.key === 'Escape') {
        e.preventDefault()
        dispatchSelection({ kind: 'escape' })
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [canEdit, selectedNodeId, editingNodeId, dispatchSelection])

  const persist = useCallback(
    (wf: WireframeFile) => {
      const json = JSON.stringify(wf, null, 2)
      setJsonText(json)
      onContentChange(json)
    },
    [onContentChange],
  )

  // --- Drag handlers ---

  const handleNodePointerDown = useCallback(
    (nodeId: string, parentId: string, e: React.PointerEvent) => {
      if (!canEdit || editingNodeId) return
      e.preventDefault()

      const pointerId = e.pointerId

      pendingRef.current = {
        nodeId,
        parentId,
        x: e.clientX,
        y: e.clientY,
      }

      const handleMove = (me: PointerEvent) => {
        if (me.pointerId !== pointerId) return
        if (!pendingRef.current) return
        const dx = me.clientX - pendingRef.current.x
        const dy = me.clientY - pendingRef.current.y
        if (Math.abs(dx) > 4 || Math.abs(dy) > 4) {
          setDraggedNodeId(pendingRef.current.nodeId)
          pendingRef.current = null
        }
      }

      const handleUp = (me: PointerEvent) => {
        if (me.pointerId !== pointerId) return
        window.removeEventListener('pointermove', handleMove)
        window.removeEventListener('pointerup', handleUp)

        if (pendingRef.current) {
          // It was a click — select the node (double-click / Enter edits text).
          const nid = pendingRef.current.nodeId
          pendingRef.current = null
          dispatchSelection({ kind: 'select-node', nodeId: nid })
        } else {
          // End of drag
          setDraggedNodeId(null)
          setDropTarget(null)
        }
      }

      window.addEventListener('pointermove', handleMove)
      window.addEventListener('pointerup', handleUp)
    },
    [canEdit, editingNodeId, dispatchSelection],
  )

  // Commit drag reorder on mouseup when dragging
  useEffect(() => {
    if (!draggedNodeId) return

    const handleUp = () => {
      const wf = wireframeRef.current
      const dt = dropTarget
      if (wf && dt && draggedNodeId) {
        const updated = reorderNode(wf, draggedNodeId, dt.parentId, dt.index)
        if (updated !== wf) {
          setWireframe(updated)
          persist(updated)
        }
      }
      setDraggedNodeId(null)
      setDropTarget(null)
    }

    window.addEventListener('pointerup', handleUp)
    return () => window.removeEventListener('pointerup', handleUp)
  }, [draggedNodeId, dropTarget, persist])

  // --- Edit handlers ---

  const handleRequestEdit = useCallback(
    (nodeId: string) => {
      dispatchSelection({ kind: 'request-edit', nodeId })
    },
    [dispatchSelection],
  )

  const handleCommitEdit = useCallback(
    (nodeId: string, value: string) => {
      dispatchSelection({ kind: 'commit-edit' })
      if (!wireframe) return
      const updated = updateNodeText(wireframe, nodeId, value)
      setWireframe(updated)
      persist(updated)
    },
    [wireframe, persist, dispatchSelection],
  )

  const handleCancelEdit = useCallback(() => {
    dispatchSelection({ kind: 'escape' })
  }, [dispatchSelection])

  const handleToggleState = useCallback(
    (nodeId: string) => {
      if (!wireframe) return
      const updated = toggleNodeState(wireframe, nodeId)
      setWireframe(updated)
      persist(updated)
    },
    [wireframe, persist],
  )

  const handleDropTargetChange = useCallback((target: DropTarget) => {
    setDropTarget(target)
  }, [])

  // Delete / duplicate the selected node. Routes through the same persist path as
  // reorder/text edits (one Y.Doc op per action), then re-points the selection:
  // cleared after delete, moved onto the new copy after duplicate.
  const handleNodeAction = useCallback(
    (action: WireframeNodeAction) => {
      const wf = wireframeRef.current
      if (!wf) return
      const seq = action === 'duplicate' ? (dupSeqRef.current += 1) : 0
      const result = applyNodeAction(wf, action, selectedNodeId, seq)
      if (!result) return
      setWireframe(result.file)
      persist(result.file)
      setSelection({ selectedNodeId: result.nextSelectedNodeId, editingNodeId: null })
    },
    [selectedNodeId, persist],
  )

  // Keyboard affordances for the selected node. The inline edit <input> handles
  // its own keys and stops propagation, so these only fire when not typing.
  useEffect(() => {
    if (!canEdit || !selectedNodeId || editingNodeId) return
    const onKeyDown = (e: KeyboardEvent) => {
      const tag = document.activeElement?.tagName
      if (tag === 'INPUT' || tag === 'TEXTAREA') return
      if (e.key === 'Delete' || e.key === 'Backspace') {
        e.preventDefault()
        handleNodeAction('delete')
      } else if ((e.key === 'd' || e.key === 'D') && (e.metaKey || e.ctrlKey)) {
        e.preventDefault()
        handleNodeAction('duplicate')
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [canEdit, selectedNodeId, editingNodeId, handleNodeAction])

  // --- JSON mode ---

  const handleJsonTextChange = (value: string) => {
    setJsonText(value)
    try {
      const parsed = JSON.parse(value)
      setJsonError(null)
      setWireframe(parsed)
      onContentChange(value)
    } catch (err) {
      setJsonError((err as Error).message)
    }
  }

  // A wireframe file with no `root` (e.g. an empty `{}` or a stub written by
  // a tool that hasn't filled it in) used to crash WireframeNodeRenderer
  // when it tried to read `node.type` on undefined. Treat it the same as a
  // parse failure — there's nothing to draw.
  if (!wireframe || !wireframe.root) {
    const message = !wireframe ? 'Invalid wireframe JSON' : 'Empty wireframe (no root)'
    return (
      <div
        style={{
          width: '100%',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: '#999',
          fontSize: 13,
          fontFamily: 'system-ui, sans-serif',
        }}
      >
        {message}
      </div>
    )
  }

  const themeName = wireframe.theme ?? 'light'
  const theme = wireframeThemes[themeName]

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        background: theme.bg,
        overflow: 'hidden',
        fontFamily: 'system-ui, sans-serif',
      }}
    >
      {jsonMode ? (
        <div style={{ flex: 1, overflow: 'auto', position: 'relative' }}>
          <textarea
            value={jsonText}
            onChange={(e) => handleJsonTextChange(e.target.value)}
            onPointerDown={(e) => e.stopPropagation()}
            spellCheck={false}
            style={{
              width: '100%',
              height: '100%',
              padding: 12,
              border: 'none',
              outline: 'none',
              resize: 'none',
              background: theme.bg,
              color: theme.text,
              fontSize: 11,
              fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
              lineHeight: 1.5,
              boxSizing: 'border-box',
            }}
          />
          {jsonError && (
            <div
              style={{
                position: 'absolute',
                bottom: 0,
                left: 0,
                right: 0,
                padding: '4px 8px',
                background: '#dc2626',
                color: '#fff',
                fontSize: 11,
                fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
              }}
            >
              {jsonError}
            </div>
          )}
        </div>
      ) : (
        <div
          style={{ flex: 1, overflow: 'auto' }}
          // Pointer events on nodes stopPropagation, so a pointerdown that reaches
          // here landed on empty canvas — clear the selection.
          onPointerDown={() => {
            if (canEdit) dispatchSelection({ kind: 'select-background' })
          }}
        >
          <WireframeNodeRenderer
            node={wireframe.root}
            theme={theme}
            canEdit={canEdit}
            draggedNodeId={draggedNodeId}
            dropTarget={dropTarget}
            selectedNodeId={selectedNodeId}
            editingNodeId={editingNodeId}
            onNodePointerDown={handleNodePointerDown}
            onDropTargetChange={handleDropTargetChange}
            onRequestEdit={handleRequestEdit}
            onCommitEdit={handleCommitEdit}
            onCancelEdit={handleCancelEdit}
            onToggleState={handleToggleState}
          />
        </div>
      )}
    </div>
  )
}
