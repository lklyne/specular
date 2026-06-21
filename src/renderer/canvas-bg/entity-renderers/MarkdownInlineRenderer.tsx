import { useEffect, useRef, useState } from 'react'
import Markdown from 'react-markdown'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import { MarkdownEditor } from '../../shared/MarkdownEditor'
import { filePathToSrc, getFileApi } from './filePathToSrc'

function renderMarkdownBody(mdContent: string | null) {
  if (mdContent == null) return <span style={{ opacity: 0.4 }}>Loading...</span>
  if (mdContent === '') return <span style={{ opacity: 0.4 }}>Write your note...</span>
  return <Markdown>{mdContent}</Markdown>
}

export function MarkdownInlineRenderer({
  entity,
  canEdit,
  isDark,
  onTextEditingChange,
}: {
  entity: CanvasSceneFileEntity
  canEdit: boolean
  isDark: boolean
  onTextEditingChange: (active: boolean) => void
}) {
  const fileApi = getFileApi()
  const [mdContent, setMdContent] = useState<string | null>(null)
  const [localText, setLocalText] = useState('')
  const isFocusedRef = useRef(false)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const flushRef = useRef<(() => void) | null>(null)

  useEffect(() => {
    let cancelled = false
    const fetchContent = () => {
      fetch(filePathToSrc(entity.file) + `?t=${Date.now()}`)
        .then((res) => res.text())
        .then((text) => {
          if (cancelled) return
          setMdContent(text)
          if (!isFocusedRef.current) setLocalText(text)
        })
        .catch(() => {
          if (!cancelled) setMdContent(null)
        })
    }
    if (!isFocusedRef.current) fetchContent()
    const handleVisibility = () => {
      if (document.visibilityState !== 'visible') return
      if (debounceRef.current) return
      if (isFocusedRef.current) return
      fetchContent()
    }
    document.addEventListener('visibilitychange', handleVisibility)
    return () => {
      cancelled = true
      document.removeEventListener('visibilitychange', handleVisibility)
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [entity.file, entity.fileReloadVersion])

  useEffect(() => {
    if (!canEdit && isFocusedRef.current) {
      isFocusedRef.current = false
      onTextEditingChange(false)
    }
  }, [canEdit, onTextEditingChange])

  // Flush a queued write if the component unmounts mid-debounce (e.g. tab
  // switch, entity deletion) — otherwise the last typed keystrokes are lost.
  useEffect(() => {
    return () => {
      if (debounceRef.current) {
        clearTimeout(debounceRef.current)
        debounceRef.current = null
        flushRef.current?.()
        flushRef.current = null
      }
    }
  }, [])

  const handleChange = (value: string) => {
    setLocalText(value)
    flushRef.current = () => fileApi.writeNoteFile(entity.file, value)
    if (debounceRef.current) clearTimeout(debounceRef.current)
    debounceRef.current = setTimeout(() => {
      flushRef.current?.()
      flushRef.current = null
      debounceRef.current = null
    }, 300)
  }

  const handleFocus = () => {
    isFocusedRef.current = true
    onTextEditingChange(true)
  }

  const handleBlur = () => {
    isFocusedRef.current = false
    onTextEditingChange(false)
    if (debounceRef.current) {
      clearTimeout(debounceRef.current)
      debounceRef.current = null
    }
    flushRef.current = null
    fileApi.writeNoteFile(entity.file, localText)
    setMdContent(localText)
  }

  const textColor = isDark ? '#e7e5e4' : '#1c1917'

  return (
    <div style={{ width: '100%', height: '100%', overflow: 'auto', padding: 12 }}>
      {canEdit ? (
        <MarkdownEditor
          value={localText}
          onChange={handleChange}
          onFocus={handleFocus}
          onBlur={handleBlur}
          isDark={isDark}
          autoFocus
          style={{
            width: '100%',
            height: '100%',
            fontSize: 14,
            lineHeight: 1.5,
            color: textColor,
          }}
        />
      ) : (
        <div
          className="text-block-markdown"
          style={{
            fontSize: 14,
            color: textColor,
            fontFamily: 'system-ui, sans-serif',
            wordBreak: 'break-word',
          }}
        >
          {renderMarkdownBody(mdContent)}
        </div>
      )}
    </div>
  )
}
