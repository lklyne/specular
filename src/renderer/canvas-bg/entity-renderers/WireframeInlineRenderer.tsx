import { useCallback, useEffect, useState } from 'react'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import { WireframeRenderer } from '../wireframe/WireframeRenderer'
import { useDebouncedWrite } from '../../shared/useDebouncedWrite'
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
  const debouncedWrite = useDebouncedWrite((json) => fileApi.writeNoteFile(entity.file, json))

  const fetchContent = useCallback(() => {
    const src = filePathToSrc(entity.file) + `?t=${Date.now()}`
    fetch(src)
      .then((res) => res.text())
      .then((text) => setContent(text))
      .catch(() => {})
  }, [entity.file])

  // Initial load + disk-change reload.
  useEffect(() => {
    if (debouncedWrite.isPending()) return // pending local write — skip
    let cancelled = false
    fetch(filePathToSrc(entity.file) + `?t=${Date.now()}`)
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
  }, [entity.file, entity.fileReloadVersion])

  // Re-fetch when window regains visibility, unless we have a pending local write.
  useEffect(() => {
    const handleVisibility = () => {
      if (document.visibilityState !== 'visible') return
      if (debouncedWrite.isPending()) return
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
      if (debouncedWrite.isPending()) return
      fetchContent()
    }
    window.addEventListener('wireframe-file-changed', handleExternalChange)
    return () => window.removeEventListener('wireframe-file-changed', handleExternalChange)
  }, [entity.file, fetchContent])

  const handleChange = useCallback(
    (json: string) => {
      setContent(json)
      debouncedWrite.schedule(json)
    },
    [debouncedWrite],
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
      />
    </div>
  )
}
