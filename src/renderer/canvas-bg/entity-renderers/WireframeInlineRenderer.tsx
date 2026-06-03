import { useCallback, useEffect, useRef, useState } from 'react'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import { WireframeRenderer } from '../wireframe/WireframeRenderer'
import { filePathToSrc, getFileApi } from './filePathToSrc'

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
  const [content, setContent] = useState<string | null>(null)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const fetchContent = useCallback(() => {
    const src = filePathToSrc(entity.file) + `?t=${Date.now()}`
    fetch(src)
      .then((res) => res.text())
      .then((text) => setContent(text))
      .catch(() => {})
  }, [entity.file])

  // Initial load.
  useEffect(() => {
    let cancelled = false
    fetch(filePathToSrc(entity.file))
      .then((res) => res.text())
      .then((text) => {
        if (!cancelled) setContent(text)
      })
      .catch(() => {
        if (!cancelled) setContent(null)
      })
    return () => {
      cancelled = true
    }
  }, [entity.file])

  // Re-fetch when window regains visibility, unless we have a pending local write.
  useEffect(() => {
    const handleVisibility = () => {
      if (document.visibilityState !== 'visible') return
      if (debounceRef.current) return
      fetchContent()
    }
    document.addEventListener('visibilitychange', handleVisibility)
    return () => document.removeEventListener('visibilitychange', handleVisibility)
  }, [fetchContent])

  // Sibling chrome (theme picker) writes to disk and dispatches this event.
  useEffect(() => {
    const handleExternalChange = (ev: Event) => {
      const detail = (ev as CustomEvent<{ file?: string }>).detail
      if (detail?.file !== entity.file) return
      if (debounceRef.current) return
      fetchContent()
    }
    window.addEventListener('wireframe-file-changed', handleExternalChange)
    return () => window.removeEventListener('wireframe-file-changed', handleExternalChange)
  }, [entity.file, fetchContent])

  // Edits applied in main from another surface (3.2: the panel insert palette)
  // ping us to re-fetch the projected JSON. Skip when we have a pending local
  // write — our own optimistic state is fresher than disk until it flushes.
  useEffect(() => {
    return fileApi.onWireframeContentChanged(({ entityId }) => {
      if (entityId !== entity.id) return
      if (debounceRef.current) return
      fetchContent()
    })
  }, [entity.id, fileApi, fetchContent])

  const handleChange = useCallback(
    (json: string) => {
      setContent(json)
      if (debounceRef.current) clearTimeout(debounceRef.current)
      debounceRef.current = setTimeout(() => {
        // 3.0b: route up as a Y.Doc op (undoable, projected to disk) rather than
        // writing the file directly.
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
