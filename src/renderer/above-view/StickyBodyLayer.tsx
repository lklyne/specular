/**
 * StickyBodyLayer — sticky-note (text entity) bodies. Mounted in aboveView
 * so a sticky placed over a page is actually drawn above it.
 *
 * Hit-tests run in `useCanvasPointerRouter` against the layout snapshot
 * (front-to-back), so this layer is purely visual for selection/drag/resize.
 * The contenteditable textarea inside is the one exception — it needs real
 * DOM events, and works because the cards mount inside aboveView's WCV
 * which already holds keyboard focus during edit.
 *
 * Plain text in `widthMode: 'auto'` hugs its content. The shell has no fixed
 * width/height; instead a ResizeObserver measures the rendered card and
 * pushes the size back to main via `onUpdateSize`, which keeps the stored
 * bounds in sync with what the user sees so the selection outline hugs
 * the text. The CodeMirror editor disables `lineWrapping` in this mode so
 * lines don't collapse to single characters when the container shrink-wraps.
 *
 * Once the user drags a resize handle, widthMode flips to 'fixed' (handled
 * by the pointer router) and the entity behaves like a sticky: explicit
 * width/height, wrap on. Stickies are always 'fixed'.
 */

import { memo, useEffect, useRef, useState } from 'react'
import Markdown from 'react-markdown'
import { PLAIN_TEXT_PLACEHOLDER } from '../../shared/constants'
import type {
  CanvasScenePageEntity,
  CanvasSceneTextEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { resolveCanvasColor } from '../../shared/canvas-colors'
import { MarkdownEditor } from '../shared/MarkdownEditor'
import { remarkLineBreaks } from '../shared/remark-line-breaks'
import { useDebouncedWrite } from '../shared/useDebouncedWrite'
import { lineHeightForTextSize } from './TextSizeDropdown'
import { CanvasViewportLayer, EntityShell } from './CanvasViewportLayer'
import { PageOverlayBand } from './PageOverlayBand'

const PLAIN_MIN_WIDTH = 64
const PLAIN_MIN_HEIGHT = 18
/** ADR 0013 §2 — entities without textSize render at this size ("Small"). */
const DEFAULT_TEXT_SIZE = 14

function stickyShellStyle({
  note,
  isDark,
  isSelected,
  isPlain,
  isAuto,
}: {
  note: CanvasSceneTextEntity
  isDark: boolean
  isSelected: boolean
  isPlain: boolean
  isAuto: boolean
}): React.CSSProperties {
  if (isPlain && isAuto) {
    return {
      // The containing viewport layer holds only absolutely-positioned
      // children, so its intrinsic width is 0. Without an explicit
      // `width`, our absolute shell's shrink-to-fit collapses to
      // `min-content` (longest word) once view mode swaps CodeMirror's
      // `white-space: pre` content for a wrapping `<p>`. `max-content`
      // pins the shell to the unwrapped line width, matching what the
      // editor showed.
      width: 'max-content',
      minWidth: PLAIN_MIN_WIDTH,
      minHeight: PLAIN_MIN_HEIGHT,
    }
  }
  if (isPlain) {
    return { width: note.width, height: note.height }
  }
  return {
    width: note.width,
    height: note.height,
    background: resolveCanvasColor(note.color, { role: 'fill', isDark, palette: 'soft' }),
    boxShadow: isDark
      ? '0 2px 8px rgba(0, 0, 0, 0.3)'
      : '0 2px 8px rgba(0, 0, 0, 0.08)',
    overflow: isSelected ? 'visible' : 'hidden',
  }
}

function StickyCard({
  note,
  isDark,
  isSelected,
  canEdit,
  onUpdateText,
  onUpdateSize,
  onCommitEdit,
}: {
  note: CanvasSceneTextEntity
  isDark: boolean
  isSelected: boolean
  canEdit: boolean
  onUpdateText: (id: string, text: string) => void
  onUpdateSize: (id: string, width: number, height: number) => void
  onCommitEdit: () => void
}) {
  const [localText, setLocalText] = useState(note.text)
  // Tracks the most recent value we sent upstream. When an incoming
  // `note.text` differs from this, we treat it as external (e.g. Yjs undo)
  // and pull it into local state — even mid-edit. When it matches, the
  // round-trip is just our own commit echoing back; ignore it so we don't
  // clobber characters typed since the last commit.
  const lastSentRef = useRef<string>(note.text)
  const shellRef = useRef<HTMLDivElement | null>(null)
  const lastReportedSizeRef = useRef<{ w: number; h: number } | null>(null)

  useEffect(() => {
    if (!canEdit) {
      lastSentRef.current = note.text
      setLocalText(note.text)
      return
    }
    if (note.text !== lastSentRef.current) {
      lastSentRef.current = note.text
      setLocalText(note.text)
    }
  }, [canEdit, note.text])

  const debouncedWrite = useDebouncedWrite((value) => {
    lastSentRef.current = value
    onUpdateText(note.id, value)
  })

  const commitNow = () => {
    debouncedWrite.cancel()
    lastSentRef.current = localText
    onUpdateText(note.id, localText)
    onCommitEdit()
  }

  const handleTextChange = (value: string) => {
    setLocalText(value)
    debouncedWrite.schedule(value)
  }

  const textStyle = note.textStyle
  const isPlain = textStyle === 'plain'
  const isAuto = note.widthMode === 'auto'

  // Auto-size: measure the rendered card and push size back to main so the
  // selection outline tracks the actual content. Only active in 'auto' mode
  // — once flipped to 'fixed' (sticky always, or after a manual resize), the
  // entity keeps its explicit width/height. Coalesces with rAF so a burst of
  // ResizeObserver entries during typing only triggers one IPC.
  useEffect(() => {
    if (!isAuto) return
    const el = shellRef.current
    if (!el) return
    let pendingFrame = 0
    let pending: { w: number; h: number } | null = null
    const flush = () => {
      pendingFrame = 0
      if (!pending) return
      const { w, h } = pending
      pending = null
      const last = lastReportedSizeRef.current
      if (last && last.w === w && last.h === h) return
      lastReportedSizeRef.current = { w, h }
      onUpdateSize(note.id, w, h)
    }
    const observer = new ResizeObserver((entries) => {
      const entry = entries[0]
      if (!entry) return
      const rect = entry.contentRect
      pending = {
        w: Math.max(PLAIN_MIN_WIDTH, Math.round(rect.width)),
        h: Math.max(PLAIN_MIN_HEIGHT, Math.round(rect.height)),
      }
      if (!pendingFrame) pendingFrame = requestAnimationFrame(flush)
    })
    observer.observe(el)
    return () => {
      observer.disconnect()
      if (pendingFrame) cancelAnimationFrame(pendingFrame)
    }
  }, [isAuto, note.id, onUpdateSize])

  // Stickies render with a light fill in both themes (neutral is pinned light
  // below via `isDark: false`), so non-plain text always uses dark ink.
  const editorIsDark = isPlain ? isDark : false
  const textColor = isPlain ? (isDark ? '#e7e5e4' : '#1c1917') : '#1c1917'
  const placeholder = isPlain ? PLAIN_TEXT_PLACEHOLDER : 'Type a note...'

  const innerColumnStyle: React.CSSProperties =
    isPlain && isAuto
      ? { display: 'flex', flexDirection: 'column' }
      : {
          width: note.width,
          height: note.height,
          display: 'flex',
          flexDirection: 'column',
        }

  const fontSize = note.textSize ?? DEFAULT_TEXT_SIZE
  // CodeMirror's `.cm-scroller` and `.text-block-markdown` both inherit
  // line-height from this wrapper, so edit and view modes stay in sync.
  const lineHeight = lineHeightForTextSize(fontSize)
  const editorClassName = isPlain
    ? 'w-full pl-0 pr-2 py-0'
    : 'flex-1 w-full px-2.5 pb-2'
  const editorStyle: React.CSSProperties = {
    boxSizing: 'border-box',
    fontSize,
    lineHeight,
    color: textColor,
    fontFamily: 'system-ui, sans-serif',
    paddingTop: isPlain ? 0 : '0.3em',
  }

  const viewClassName = isPlain
    ? 'select-none text-block-markdown pr-2'
    : 'flex-1 select-none overflow-hidden text-block-markdown px-2 pb-2'
  const viewStyle: React.CSSProperties = {
    fontSize,
    lineHeight,
    color: textColor,
    fontFamily: 'system-ui, sans-serif',
    wordBreak: 'break-word',
  }

  return (
    <EntityShell
      id={note.id}
      canvasX={note.canvasX}
      canvasY={note.canvasY}
      style={stickyShellStyle({ note, isDark, isSelected, isPlain, isAuto })}
      shellRef={shellRef}
    >
      <div style={innerColumnStyle}>
        {!isPlain ? (
          <div
            style={{ minHeight: 8, cursor: 'grab' }}
            onPointerDown={(e) => {
              if (e.button !== 0) return
              e.stopPropagation()
            }}
          />
        ) : null}
        {canEdit ? (
          <MarkdownEditor
            value={localText}
            onChange={handleTextChange}
            onBlur={commitNow}
            onEscape={commitNow}
            isDark={editorIsDark}
            autoFocus
            placeholder={placeholder}
            className={editorClassName}
            style={editorStyle}
            lineWrap={!isAuto}
          />
        ) : (
          <div className={viewClassName} style={viewStyle}>
            {localText ? (
              <Markdown remarkPlugins={[remarkLineBreaks]}>{localText}</Markdown>
            ) : (
              <span>{placeholder}</span>
            )}
          </div>
        )}
      </div>
    </EntityShell>
  )
}

const MemoStickyCard = memo(StickyCard, (prev, next) => {
  return (
    prev.note.id === next.note.id &&
    prev.note.text === next.note.text &&
    prev.note.color === next.note.color &&
    prev.note.textStyle === next.note.textStyle &&
    prev.note.widthMode === next.note.widthMode &&
    prev.note.textSize === next.note.textSize &&
    prev.note.canvasX === next.note.canvasX &&
    prev.note.canvasY === next.note.canvasY &&
    prev.note.width === next.note.width &&
    prev.note.height === next.note.height &&
    prev.isDark === next.isDark &&
    prev.isSelected === next.isSelected &&
    prev.canEdit === next.canEdit
  )
})

export function StickyBodyLayer({
  entities,
  isDark,
  selectedEntityIdSet,
  editingEntityId,
  layoutData,
  onUpdateText,
  onUpdateSize,
  onCommitEdit,
}: {
  entities: CanvasSceneTextEntity[]
  isDark: boolean
  selectedEntityIdSet: Set<string>
  /** id of the entity currently in inline-edit mode (or null). Mounts the
   *  editor iff `editingEntityId === note.id`. */
  editingEntityId: string | null
  layoutData: LayoutUpdateData
  onUpdateText: (id: string, text: string) => void
  onUpdateSize: (id: string, width: number, height: number) => void
  onCommitEdit: () => void
}) {
  if (!entities.length) return null
  const viewport = (
    <CanvasViewportLayer
      canvasOrigin={layoutData.canvasOrigin}
      pan={layoutData.pan}
      zoom={layoutData.zoom}
    >
      {entities.map((note) => (
        <MemoStickyCard
          key={note.id}
          note={note}
          isDark={isDark}
          isSelected={selectedEntityIdSet.has(note.id)}
          canEdit={editingEntityId === note.id}
          onUpdateText={onUpdateText}
          onUpdateSize={onUpdateSize}
          onCommitEdit={onCommitEdit}
        />
      ))}
    </CanvasViewportLayer>
  )
  // A page-anchored text scroll-follows its page (main shifts the scene
  // coords), so it clips and edge-fades inside the page's overlay band like
  // shapes do. App mounts one layer per entity, so the single entity's
  // anchor decides the wrapping.
  const anchorPageId = entities[0].pageAnchor?.pageId
  const page = anchorPageId
    ? layoutData.entities.find(
        (entity): entity is CanvasScenePageEntity =>
          entity.kind === 'page' && entity.id === anchorPageId,
      )
    : undefined
  if (!page) return viewport
  return (
    <PageOverlayBand page={page} layoutData={layoutData} followScroll>
      {viewport}
    </PageOverlayBand>
  )
}
