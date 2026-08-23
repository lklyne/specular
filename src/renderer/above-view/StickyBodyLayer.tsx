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

import type { ProjectedLayoutData } from '../../shared/scene-projection'
import { memo, useEffect, useRef, useState } from 'react'
import { PLAIN_TEXT_PLACEHOLDER, STICKY_BASE_HEIGHT } from '../../shared/constants'
import { useMeasuredSize } from '../shared/useMeasuredSize'
import type { CanvasSceneTextEntity } from '../../shared/types'
import { resolveCanvasColor } from '../../shared/canvas-colors'
import { MarkdownEditor } from '../shared/MarkdownEditor'
import { useDebouncedWrite } from '../shared/useDebouncedWrite'
import { lineHeightForTextSize } from './TextSizeDropdown'
import { CanvasViewportLayer, EntityShell } from './CanvasViewportLayer'
import { AnchoredEntityOverlayBand } from './PageOverlayBand'
import { useEditorBridge } from '../shared/markdown/text-editor-bridge'

const PLAIN_MIN_WIDTH = 64
const PLAIN_MIN_HEIGHT = 18
/** ADR 0013 §2 — entities without textSize render at this size ("Small"). */
const DEFAULT_TEXT_SIZE = 14

function stickyShellStyle({
  note,
  isDark,
  isPlain,
  isAuto,
}: {
  note: CanvasSceneTextEntity
  isDark: boolean
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
  // A sticky is width-driven: height grows with the text so it always contains
  // it and never scrolls. `note.height` is the measured content height —
  // `contentHeightLayout` has already patched it into this layout, so the card
  // and the selection outline are drawing the same rect, not two roundings of
  // the same idea.
  return {
    width: note.width,
    height: note.height,
    background: resolveCanvasColor(note.color, { role: 'fill', isDark, palette: 'soft' }),
    boxShadow: isDark
      ? '0 2px 8px rgba(0, 0, 0, 0.3)'
      : '0 2px 8px rgba(0, 0, 0, 0.08)',
  }
}

function StickyCard({
  note,
  isDark,
  canEdit,
  onUpdateText,
  onUpdateSize,
  onContentHeight,
  onCommitEdit,
}: {
  note: CanvasSceneTextEntity
  isDark: boolean
  canEdit: boolean
  onUpdateText: (id: string, text: string) => void
  onUpdateSize: (id: string, width: number, height: number) => void
  onContentHeight: (id: string, height: number) => void
  onCommitEdit: () => void
}) {
  const shellRef = useRef<HTMLDivElement | null>(null)
  const contentRef = useRef<HTMLDivElement | null>(null)
  const { localText, handleTextChange, commitNow } = useStickyText({
    note,
    canEdit,
    onUpdateText,
    onCommitEdit,
  })
  const isPlain = note.textStyle === 'plain'
  const isAuto = note.widthMode === 'auto'
  useStickyAutoSize(shellRef, isAuto, note, onUpdateSize)
  useStickyHeight(contentRef, !isPlain, note, onContentHeight)

  return (
    <EntityShell
      id={note.id}
      canvasX={note.canvasX}
      canvasY={note.canvasY}
      style={stickyShellStyle({ note, isDark, isPlain, isAuto })}
      shellRef={shellRef}
    >
      <StickyContent
        note={note}
        contentRef={contentRef}
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
      // Escape commits and then main flips the editor read-only, which blurs
      // it and commits again. Re-sending identical text would be a second
      // Y.Doc transaction, i.e. an extra undo step for one edit.
      if (localText !== lastSentRef.current) {
        lastSentRef.current = localText
        onUpdateText(note.id, localText)
      }
      onCommitEdit()
    },
  }
}

/**
 * Plain text in `widthMode: 'auto'` hugs its content on both axes: measure the
 * shell (itself content-sized) and report that as the stored bounds. Unsnapped
 * — auto text ends wherever the glyphs end.
 */
function useStickyAutoSize(
  shellRef: React.MutableRefObject<HTMLDivElement | null>,
  enabled: boolean,
  note: CanvasSceneTextEntity,
  onUpdateSize: (id: string, width: number, height: number) => void,
): void {
  const measured = useMeasuredSize(shellRef, enabled)
  const width = measured ? Math.max(PLAIN_MIN_WIDTH, Math.round(measured.width)) : null
  const height = measured ? Math.max(PLAIN_MIN_HEIGHT, Math.round(measured.height)) : null
  useEffect(() => {
    if (!enabled || width === null || height === null) return
    if (width === note.width && height === note.height) return
    onUpdateSize(note.id, width, height)
  }, [enabled, width, height, note.id, note.width, note.height, onUpdateSize])
}

/**
 * A sticky's height is exactly its text's height — publish it and let
 * `contentHeightLayout` feed it back through the layout every layer reads.
 *
 * Deliberately *not* grid-snapped. Grid snapping is a policy for edges the
 * user drags, so they land on the grid; a measured size snapped up carries
 * up to a full grid step of dead space under the last line. Ceil to whole
 * pixels only — a fractional height rounded down clips the descenders.
 *
 * Measures the inner content column (auto-height); the shell it feeds is
 * explicitly sized, so observing the shell would be a fixed point that never
 * notices the text growing.
 */
function useStickyHeight(
  contentRef: React.MutableRefObject<HTMLDivElement | null>,
  enabled: boolean,
  note: CanvasSceneTextEntity,
  onContentHeight: (id: string, height: number) => void,
): void {
  const measured = useMeasuredSize(contentRef, enabled)
  // Floored so an empty note stays note-shaped rather than collapsing to one
  // line of padding. The floor scales with the text because that is what a
  // corner or n/s drag does to a sticky — hold it fixed and the box stops
  // shrinking while the font keeps going, so the note can never get smaller
  // than the size it was created at. A floor tracking the *width* instead
  // would make every wide note tall, which is what a reflow is trying to undo.
  const floor = (STICKY_BASE_HEIGHT * (note.textSize ?? DEFAULT_TEXT_SIZE)) / DEFAULT_TEXT_SIZE
  const height = measured ? Math.ceil(Math.max(floor, measured.height)) : null
  useEffect(() => {
    if (height === null) return
    onContentHeight(note.id, height)
  }, [height, note.id, onContentHeight])
}

function StickyContent({ note, contentRef, isDark, canEdit, isPlain, isAuto, localText, onChange, onCommit }: {
  note: CanvasSceneTextEntity
  contentRef: React.MutableRefObject<HTMLDivElement | null>
  isDark: boolean
  canEdit: boolean
  isPlain: boolean
  isAuto: boolean
  localText: string
  onChange: (value: string) => void
  onCommit: () => void
}) {
  const fontSize = note.textSize ?? DEFAULT_TEXT_SIZE
  const editorBridge = useEditorBridge(note.id, canEdit)
  const columnStyle: React.CSSProperties = isPlain && isAuto
    ? { display: 'flex', flexDirection: 'column' }
    : isPlain
      ? { width: note.width, height: note.height, display: 'flex', flexDirection: 'column' }
      // Sticky: width fixed, height follows the text (see stickyShellStyle).
      : { width: note.width, display: 'flex', flexDirection: 'column' }
  return <div ref={contentRef} style={columnStyle}>
    {!isPlain && <StickyDragStrip />}
    {/* One renderer for both modes: same padding, same line boxes, so
        entering and leaving edit mode can't reflow the text. */}
    <MarkdownEditor
      variant="sticky"
      readOnly={!canEdit}
      value={localText}
      onChange={onChange}
      onBlur={onCommit}
      onEscape={onCommit}
      onViewReady={editorBridge.onViewReady}
      onSelectionChange={editorBridge.onSelectionChange}
      isDark={isPlain && isDark}
      autoFocus={canEdit}
      selectAllOnAutoFocus
      placeholder={isPlain ? PLAIN_TEXT_PLACEHOLDER : 'Type a note...'}
      className={isPlain ? 'w-full pl-0 pr-2 py-0' : 'flex-1 w-full px-2 pb-2'}
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
    prev.canEdit === next.canEdit
  )
})

export const StickyBodyLayer = memo(function StickyBodyLayer({
  entities,
  isDark,
  editingEntityId,
  layoutData,
  onUpdateText,
  onUpdateSize,
  onContentHeight,
  onCommitEdit,
}: {
  entities: CanvasSceneTextEntity[]
  isDark: boolean
  /** id of the entity currently in inline-edit mode (or null). Mounts the
   *  editor iff `editingEntityId === note.id`. */
  editingEntityId: string | null
  layoutData: ProjectedLayoutData
  onUpdateText: (id: string, text: string) => void
  onUpdateSize: (id: string, width: number, height: number) => void
  /** Publishes a sticky's measured height (see `contentHeightPreview.ts`). */
  onContentHeight: (id: string, height: number) => void
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
          canEdit={editingEntityId === note.id}
          onUpdateText={onUpdateText}
          onUpdateSize={onUpdateSize}
          onContentHeight={onContentHeight}
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
})
