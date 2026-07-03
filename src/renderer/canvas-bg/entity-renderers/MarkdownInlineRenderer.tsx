import { useEffect, useRef, useState } from 'react'
import Markdown from 'react-markdown'
import type { CanvasSceneFileEntity } from '../../../shared/types'
import { MarkdownEditor } from '../../shared/MarkdownEditor'
import { useDebouncedWrite } from '../../shared/useDebouncedWrite'
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
  // Flush-on-unmount so a queued write mid-debounce (e.g. tab switch, entity
  // deletion) isn't lost — otherwise the last typed keystrokes disappear.
  const debouncedWrite = useDebouncedWrite(
    (value) => fileApi.writeNoteFile(entity.file, value),
    { flushOnUnmount: true },
  )

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
    fetchContent()
    const handleVisibility = () => {
      if (document.visibilityState !== 'visible') return
      if (debouncedWrite.isPending()) return
      if (isFocusedRef.current) return
      fetchContent()
    }
    document.addEventListener('visibilitychange', handleVisibility)
    return () => {
      cancelled = true
      document.removeEventListener('visibilitychange', handleVisibility)
    }
  }, [entity.file])

  useEffect(() => {
    if (!canEdit && isFocusedRef.current) {
      isFocusedRef.current = false
      onTextEditingChange(false)
    }
  }, [canEdit, onTextEditingChange])

  const handleChange = (value: string) => {
    setLocalText(value)
    debouncedWrite.schedule(value)
  }

  const handleFocus = () => {
    isFocusedRef.current = true
    onTextEditingChange(true)
  }

  const handleBlur = () => {
    isFocusedRef.current = false
    onTextEditingChange(false)
    debouncedWrite.cancel()
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
