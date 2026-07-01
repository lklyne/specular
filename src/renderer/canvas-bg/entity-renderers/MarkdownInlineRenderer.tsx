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
  const [mdContent, setMdContent] = useState<string | null>(entity.noteContent ?? null)
  const [localText, setLocalText] = useState(entity.noteContent ?? '')
  const isFocusedRef = useRef(false)
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null)
  const flushRef = useRef<(() => void) | null>(null)
  const dirtyRef = useRef(false)
  // Mirrors StickyBodyLayer's echo-suppression: once the note has entered
  // the Y.Doc mirror, `entity.noteContent` broadcasts on every scene
  // rebuild. Track what we last sent upstream so our own commit echoing
  // back doesn't clobber characters typed since; anything else (Yjs
  // undo/redo) is external and pulled in even mid-edit.
  const lastSentRef = useRef<string | null>(entity.noteContent ?? null)

  // Initial disk load — only while the note hasn't entered the Y.Doc mirror
  // yet (entity.noteContent undefined, i.e. never edited by anyone since
  // the workspace loaded). Once tracked, scene broadcasts are authoritative.
  useEffect(() => {
    if (entity.noteContent !== undefined) return
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
      if (debounceRef.current) return
      if (isFocusedRef.current) return
      fetchContent()
    }
    document.addEventListener('visibilitychange', handleVisibility)
    return () => {
      cancelled = true
      document.removeEventListener('visibilitychange', handleVisibility)
    }
  }, [entity.file, entity.noteContent])

  // Reflect Y.Doc-driven changes (undo/redo) once the note is tracked.
  useEffect(() => {
    if (entity.noteContent === undefined) return
    setMdContent(entity.noteContent)
    if (entity.noteContent !== lastSentRef.current) {
      lastSentRef.current = entity.noteContent
      setLocalText(entity.noteContent)
      dirtyRef.current = false
    }
  }, [entity.noteContent])

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

  const commit = (value: string) => {
    lastSentRef.current = value
    fileApi.applyNoteContent(entity.id, value)
  }

  const handleChange = (value: string) => {
    dirtyRef.current = true
    setLocalText(value)
    flushRef.current = () => {
      commit(value)
      dirtyRef.current = false
    }
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
    if (dirtyRef.current) {
      commit(localText)
      dirtyRef.current = false
    }
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
