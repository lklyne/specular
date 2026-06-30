import { useCallback, useEffect, useRef, useState } from 'react'
import { withDragLifecycle } from '../../../shared/drag-lifecycle'
import type { WireframeFile, WireframeThemeName, DropTarget } from './wireframe-types'
import { wireframeThemes } from './wireframe-themes'
import { WireframeNodeRenderer } from './WireframeNodeRenderer'
import {
  reorderNode,
  updateNodeText,
  toggleNodeState,
  findNodeById,
  nodeHasEditableText,
} from './wireframe-utils'

export const WIREFRAME_THEME_OPTIONS: { name: WireframeThemeName; color: string }[] = [
  { name: 'light', color: '#ffffff' },
  { name: 'dark', color: '#18181b' },
  { name: 'blueprint', color: '#0f2744' },
]

export function WireframeRenderer({
  content,
  canEdit,
  jsonMode = false,
  onContentChange,
}: {
  content: string
  canEdit: boolean
  jsonMode?: boolean
  onContentChange: (json: string) => void
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
  const [editingNodeId, setEditingNodeId] = useState<string | null>(null)

  const wireframeRef = useRef(wireframe)
  wireframeRef.current = wireframe

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

  // Clear edit state when edit mode is lost
  useEffect(() => {
    if (!canEdit) {
      setEditingNodeId(null)
      setDraggedNodeId(null)
      setDropTarget(null)
    }
  }, [canEdit])

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
    (nodeId: string, _parentId: string, e: React.PointerEvent) => {
      if (!canEdit || editingNodeId) return
      e.preventDefault()

      withDragLifecycle(e.nativeEvent, {
        // The wireframe's nested per-node `onPointerMove` (in
        // WireframeNodeRenderer) drives the drop-target preview once
        // `draggedNodeId` is set — this lifecycle only needs to flip that
        // state once, at the threshold crossing.
        skipBlurCancel: true,
        onMove: () => setDraggedNodeId(nodeId),
        onCommit: (_event, dragging) => {
          if (dragging) {
            setDraggedNodeId(null)
            setDropTarget(null)
            return
          }
          // It was a click — trigger edit if applicable.
          const wf = wireframeRef.current
          if (wf) {
            const node = findNodeById(wf.root, nodeId)
            if (node && nodeHasEditableText(node)) {
              setEditingNodeId(nodeId)
            }
          }
        },
        onCancel: () => {
          setDraggedNodeId(null)
          setDropTarget(null)
        },
      })
    },
    [canEdit, editingNodeId],
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

  const handleStartEdit = useCallback((nodeId: string) => {
    setEditingNodeId(nodeId)
  }, [])

  const handleCommitEdit = useCallback(
    (nodeId: string, value: string) => {
      setEditingNodeId(null)
      if (!wireframe) return
      const updated = updateNodeText(wireframe, nodeId, value)
      setWireframe(updated)
      persist(updated)
    },
    [wireframe, persist],
  )

  const handleCancelEdit = useCallback(() => {
    setEditingNodeId(null)
  }, [])

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
        <div style={{ flex: 1, overflow: 'auto' }}>
          <WireframeNodeRenderer
            node={wireframe.root}
            theme={theme}
            canEdit={canEdit}
            draggedNodeId={draggedNodeId}
            dropTarget={dropTarget}
            editingNodeId={editingNodeId}
            onNodePointerDown={handleNodePointerDown}
            onDropTargetChange={handleDropTargetChange}
            onStartEdit={handleStartEdit}
            onCommitEdit={handleCommitEdit}
            onCancelEdit={handleCancelEdit}
            onToggleState={handleToggleState}
          />
        </div>
      )}
    </div>
  )
}
