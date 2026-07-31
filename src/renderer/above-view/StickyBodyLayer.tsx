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
 * Both view and edit mode render through the same `MarkdownEditor`
 * (read-only when not editing), so the two modes share one set of padding
 * and line boxes and the mode swap can't reflow the text.
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
import { PLAIN_TEXT_PLACEHOLDER } from '../../shared/constants'
import type { CanvasSceneTextEntity, LayoutUpdateData } from '../../shared/types'
import { resolveCanvasColor } from '../../shared/canvas-colors'
import { MarkdownEditor } from '../shared/MarkdownEditor'
import { useDebouncedWrite } from '../shared/useDebouncedWrite'
import { lineHeightForTextSize } from './TextSizeDropdown'
import { CanvasViewportLayer, EntityShell } from './CanvasViewportLayer'
import { AnchoredEntityOverlayBand } from './PageOverlayBand'

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
      // `min-content` (longest word). `max-content` pins the shell to the
      // unwrapped line width of CodeMirror's `white-space: pre` content.
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
  const shellRef = useRef<HTMLDivElement | null>(null)
  const { localText, handleTextChange, commitNow } = useStickyText({
    note,
    canEdit,
    onUpdateText,
    onCommitEdit,
  })
  const isPlain = note.textStyle === 'plain'
  const isAuto = note.widthMode === 'auto'
  useStickyAutoSize(shellRef, isAuto, note.id, onUpdateSize)

  return (
    <EntityShell
      id={note.id}
      canvasX={note.canvasX}
      canvasY={note.canvasY}
      style={stickyShellStyle({ note, isDark, isSelected, isPlain, isAuto })}
      shellRef={shellRef}
    >
      <StickyContent
        note={note}
        isDark={isDark}
        canEdit={canEdit}
        isPlain={isPlain}
        isAuto={isAuto}
        localText={localText}
        onChange={handleTextChange}
        onCommit={commitNow}
      />
    </EntityShell>
  )
}

function useStickyText({ note, canEdit, onUpdateText, onCommitEdit }: {
  note: CanvasSceneTextEntity
  canEdit: boolean
  onUpdateText: (id: string, text: string) => void
  onCommitEdit: () => void
}) {
  const [localText, setLocalText] = useState(note.text)
  const lastSentRef = useRef(note.text)
  useEffect(() => {
    if (!canEdit || note.text !== lastSentRef.current) {
      lastSentRef.current = note.text
      setLocalText(note.text)
    }
  }, [canEdit, note.text])
  const debouncedWrite = useDebouncedWrite((value) => {
    lastSentRef.current = value
    onUpdateText(note.id, value)
  })
  return {
    localText,
    handleTextChange: (value: string) => {
      setLocalText(value)
      debouncedWrite.schedule(value)
    },
    commitNow: () => {
      debouncedWrite.cancel()
      lastSentRef.current = localText
      onUpdateText(note.id, localText)
      onCommitEdit()
    },
  }
}

function useStickyAutoSize(
  shellRef: React.MutableRefObject<HTMLDivElement | null>,
  isAuto: boolean,
  noteId: string,
  onUpdateSize: (id: string, width: number, height: number) => void,
): void {
  const lastReportedSizeRef = useRef<{ w: number; h: number } | null>(null)
  useEffect(() => {
    const el = shellRef.current
    if (!isAuto || !el) return
    let pendingFrame = 0
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return
      if (pendingFrame) cancelAnimationFrame(pendingFrame)
      pendingFrame = requestAnimationFrame(() => {
        const size = {
          w: Math.max(PLAIN_MIN_WIDTH, Math.round(entry.contentRect.width)),
          h: Math.max(PLAIN_MIN_HEIGHT, Math.round(entry.contentRect.height)),
        }
        const last = lastReportedSizeRef.current
        if (last?.w === size.w && last.h === size.h) return
        lastReportedSizeRef.current = size
        onUpdateSize(noteId, size.w, size.h)
      })
    })
    observer.observe(el)
    return () => {
      observer.disconnect()
      if (pendingFrame) cancelAnimationFrame(pendingFrame)
    }
  }, [isAuto, noteId, onUpdateSize, shellRef])
}

function StickyContent({ note, isDark, canEdit, isPlain, isAuto, localText, onChange, onCommit }: {
  note: CanvasSceneTextEntity
  isDark: boolean
  canEdit: boolean
  isPlain: boolean
  isAuto: boolean
  localText: string
  onChange: (value: string) => void
  onCommit: () => void
}) {
  const fontSize = note.textSize ?? DEFAULT_TEXT_SIZE
  const columnStyle: React.CSSProperties = isPlain && isAuto
    ? { display: 'flex', flexDirection: 'column' }
    : { width: note.width, height: note.height, display: 'flex', flexDirection: 'column' }
  return <div style={columnStyle}>
    {!isPlain && <StickyDragStrip />}
    {/* One renderer for both modes: same padding, same line boxes, so
        entering and leaving edit mode can't reflow the text. `key` forces
        the remount MarkdownEditor's mount-time `readOnly` needs. */}
    <MarkdownEditor
      key={canEdit ? 'edit' : 'view'}
      readOnly={!canEdit}
      value={localText}
      onChange={onChange}
      onBlur={onCommit}
      onEscape={onCommit}
      isDark={isPlain && isDark}
      autoFocus={canEdit}
      selectAllOnAutoFocus
      placeholder={isPlain ? PLAIN_TEXT_PLACEHOLDER : 'Type a note...'}
      className={isPlain ? 'w-full pl-0 pr-2 py-0' : 'flex-1 w-full overflow-hidden px-2 pb-2'}
      style={{
        fontSize,
        lineHeight: lineHeightForTextSize(fontSize),
        color: isPlain && isDark ? '#e7e5e4' : '#1c1917',
        fontFamily: 'system-ui, sans-serif',
        boxSizing: 'border-box',
      }}
      lineWrap={!isAuto}
    />
  </div>
}

function StickyDragStrip() {
  return <div style={{ minHeight: 8, cursor: 'grab' }} onPointerDown={(event) => {
    if (event.button === 0) event.stopPropagation()
  }} />
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
  const anchor = entities[0].pageAnchor
  return (
    <AnchoredEntityOverlayBand anchor={anchor} layoutData={layoutData}>
      {viewport}
    </AnchoredEntityOverlayBand>
  )
}
