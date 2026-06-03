import { useCallback, useEffect, useRef, useState } from 'react'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import { WireframeRenderer } from '../wireframe/WireframeRenderer'
import { getFileApi } from './filePathToSrc'

export function WireframeInlineRenderer({
  entity,
  canEdit,
  isDark,
  jsonMode,
}: {
  entity: CanvasSceneFileEntity
  canEdit: boolean
  isDark: boolean
  jsonMode: boolean
}) {
  const fileApi = getFileApi()
  // 3.5b: the canonical content rides the scene broadcast. The renderer derives
  // its tree from it instead of fetching the file — "renderer state is derived
  // from broadcasts, never authoritative."
  const broadcastContent = entity.wireframeContent ?? null
  const [content, setContent] = useState<string | null>(broadcastContent)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Adopt broadcast content unless a local write is in flight — our optimistic
  // state is fresher than the broadcast until it round-trips back through the
  // Y.Doc apply path.
  useEffect(() => {
    if (debounceRef.current) return
    setContent(broadcastContent)
  }, [broadcastContent])

  const handleChange = useCallback(
    (json: string) => {
      setContent(json)
      if (debounceRef.current) clearTimeout(debounceRef.current)
      debounceRef.current = setTimeout(() => {
        // Route up as a Y.Doc op (undoable, projected to disk) rather than
        // writing the file directly. The commit re-broadcasts the scene.
        fileApi.applyWireframeContent(entity.id, json)
        debounceRef.current = null
      }, 300)
    },
    [entity.id, fileApi],
  )

  if (content == null) {
    return (
      <div
        style={{
          width: '100%',
          height: '100%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: isDark ? '#a8a29e' : '#78716c',
          fontSize: 13,
          fontFamily: 'system-ui, sans-serif',
        }}
      >
        Loading...
      </div>
    )
  }

  return (
    <div style={{ width: '100%', height: '100%', pointerEvents: canEdit ? 'auto' : 'none' }}>
      <WireframeRenderer
        content={content}
        canEdit={canEdit}
        jsonMode={jsonMode && canEdit}
        onContentChange={handleChange}
        onSelectionChange={(node) => fileApi.setWireframeSelection(entity.id, node)}
      />
    </div>
  )
}
