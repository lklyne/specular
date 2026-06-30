/**
 * ShapeBodyLayer — shape (rectangle / ellipse / diamond) bodies. Mounted
 * in aboveView so a shape placed over a page is actually drawn above it.
 *
 * Hit-tests run in `useCanvasPointerRouter` against the layout snapshot
 * (front-to-back), so this layer is purely visual for selection/drag/resize.
 * The contenteditable label inside is the one exception — it needs real
 * DOM events, and works because the cards mount inside aboveView's WCV
 * which already holds keyboard focus during edit.
 */

import { memo, useEffect, useLayoutEffect, useRef, useState } from 'react'
import type { CanvasSceneShapeEntity } from '../../shared/types'
import { lightenHex, resolveCanvasColor } from '../../shared/canvas-colors'
import { CanvasViewportLayer } from './CanvasViewportLayer'

const DEFAULT_STROKE_WIDTH = 2
/** ADR 0013 §2 — shapes without textSize render their label at this size. */
const DEFAULT_TEXT_SIZE = 14
const FILL_LIGHTEN = 0.5
const NEUTRAL_SLATE = '#6b7280'

function ShapeText({
  text,
  editing,
  textColor,
  fontSize,
  onChange,
  onCommit,
  containerStyle,
}: {
  text: string
  editing: boolean
  textColor: string
  fontSize: number
  onChange: (value: string) => void
  onCommit: (value: string) => void
  containerStyle: React.CSSProperties
}) {
  const ref = useRef<HTMLDivElement>(null)

  // Keep DOM textContent in sync with prop while not editing. Avoids
  // overwriting the user's in-flight typing. useLayoutEffect runs before
  // paint so the initial mount never shows an empty page.
  useLayoutEffect(() => {
    if (!editing && ref.current && ref.current.textContent !== text) {
      ref.current.textContent = text
    }
  }, [text, editing])

  // On entering edit mode, focus and select all.
  useEffect(() => {
    const node = ref.current
    if (!editing || !node) return
    node.focus()
    const range = document.createRange()
    range.selectNodeContents(node)
    const sel = window.getSelection()
    sel?.removeAllRanges()
    sel?.addRange(range)
  }, [editing])

  return (
    <div style={containerStyle}>
      <div
        ref={ref}
        contentEditable={editing}
        suppressContentEditableWarning
        onInput={(e) => onChange((e.target as HTMLDivElement).textContent ?? '')}
        onPointerDown={(e) => { if (editing) e.stopPropagation() }}
        onBlur={(e) => {
          onCommit((e.target as HTMLDivElement).textContent ?? '')
        }}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.preventDefault()
            onCommit((e.target as HTMLDivElement).textContent ?? '')
          }
        }}
        style={{
          width: '100%',
          maxHeight: '100%',
          fontSize,
          lineHeight: 1.4,
          color: textColor,
          fontFamily: 'system-ui, sans-serif',
          textAlign: 'center',
          overflow: 'hidden',
          wordBreak: 'break-word',
          whiteSpace: 'pre-wrap',
          outline: 'none',
          userSelect: editing ? 'text' : 'none',
          pointerEvents: editing ? 'auto' : 'none',
          cursor: editing ? 'text' : 'inherit',
        }}
      />
    </div>
  )
}

function ShapeBody({
  shape,
  isDark,
  editing,
  onCommitText,
  onCommitEdit,
}: {
  shape: CanvasSceneShapeEntity
  isDark: boolean
  /** True when this shape is the active edit-mode entity. */
  editing: boolean
  onCommitText: (text: string) => void
  onCommitEdit: () => void
  selected: boolean
}) {
  const [localText, setLocalText] = useState(shape.text)
  const localTextRef = useRef(localText)
  localTextRef.current = localText
  const onCommitTextRef = useRef(onCommitText)
  onCommitTextRef.current = onCommitText
  const wasEditingRef = useRef(editing)

  // Two responsibilities, split so external `shape.text` updates can't
  // trigger a buffered-text flush:
  //
  // 1. On the editing → false TRANSITION, flush any unsaved local text.
  //    The contentEditable's onBlur is the normal commit path, but the
  //    outside-click router preventDefaults the pointerdown that would
  //    have caused the blur, so onBlur can be skipped entirely on
  //    external commits. This catches that case.
  //
  // 2. While not editing, mirror external `shape.text` changes (e.g.
  //    Yjs undo) into local state. Previously, a single effect did both
  //    and would re-commit `localText` whenever an external undo changed
  //    `shape.text` — undoing the undo and corrupting the undo stack.
  useEffect(() => {
    const wasEditing = wasEditingRef.current
    wasEditingRef.current = editing
    if (wasEditing && !editing) {
      if (localTextRef.current !== shape.text) {
        onCommitTextRef.current(localTextRef.current)
      }
    }
  }, [editing, shape.text])

  useEffect(() => {
    if (editing) return
    if (localTextRef.current === shape.text) return
    setLocalText(shape.text)
  }, [editing, shape.text])

  const stroke = shape.strokeWidth ?? DEFAULT_STROKE_WIDTH
  const resolvedColor = shape.color
    ? resolveCanvasColor(shape.color, { role: 'fill', isDark, palette: 'soft' })
    : NEUTRAL_SLATE
  // Opaque fill — the resolved hue lightened toward white, no alpha.
  const fill = lightenHex(resolvedColor, FILL_LIGHTEN)
  const strokeColor = resolvedColor
  const textColor = isDark ? 'rgb(220, 220, 220)' : 'rgb(20, 20, 20)'

  const baseStyle: React.CSSProperties = {
    width: '100%',
    height: '100%',
    boxSizing: 'border-box',
    borderWidth: stroke,
    borderStyle: 'solid',
    borderColor: strokeColor,
    backgroundColor: fill,
    pointerEvents: 'none',
  }

  const textContainerStyle: React.CSSProperties =
    shape.shapeKind === 'diamond'
      ? {
          position: 'absolute',
          left: '25%',
          top: '25%',
          width: '50%',
          height: '50%',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: 8,
          boxSizing: 'border-box',
          pointerEvents: editing ? 'auto' : 'none',
        }
      : {
          position: 'absolute',
          inset: 0,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          padding: 8,
          pointerEvents: editing ? 'auto' : 'none',
        }

  const text = (
    <ShapeText
      text={localText}
      editing={editing}
      textColor={textColor}
      fontSize={shape.textSize ?? DEFAULT_TEXT_SIZE}
      containerStyle={textContainerStyle}
      onChange={setLocalText}
      onCommit={(value) => {
        setLocalText(value)
        onCommitText(value)
        onCommitEdit()
      }}
    />
  )

  if (shape.shapeKind === 'rectangle') {
    return (
      <>
        <div style={baseStyle} />
        {text}
      </>
    )
  }

  if (shape.shapeKind === 'ellipse') {
    return (
      <>
        <div style={{ ...baseStyle, borderRadius: '50%' }} />
        {text}
      </>
    )
  }

  // diamond: a true rhombus with vertices at the midpoints of each
  // bounding-box edge. Drawn as an SVG polygon so the stroke stays uniform
  // when the bounding box is non-square. The text box is the largest
  // axis-aligned rectangle that fits inside the rhombus — half the bbox
  // width × half the height.
  return (
    <>
      <svg
        width="100%"
        height="100%"
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
        style={{ position: 'absolute', inset: 0, pointerEvents: 'none', overflow: 'visible' }}
      >
        <polygon
          points="50,0 100,50 50,100 0,50"
          fill={fill}
          stroke={strokeColor}
          strokeWidth={stroke}
          vectorEffect="non-scaling-stroke"
        />
      </svg>
      {text}
    </>
  )
}

const MemoShapeBody = memo(ShapeBody, (a, b) => {
  return (
    a.shape.id === b.shape.id &&
    a.shape.shapeKind === b.shape.shapeKind &&
    a.shape.text === b.shape.text &&
    a.shape.color === b.shape.color &&
    a.shape.strokeWidth === b.shape.strokeWidth &&
    a.shape.textSize === b.shape.textSize &&
    a.shape.width === b.shape.width &&
    a.shape.height === b.shape.height &&
    a.isDark === b.isDark &&
    a.editing === b.editing &&
    a.selected === b.selected
  )
})

function ShapeShell({
  id,
  canvasX,
  canvasY,
  width,
  height,
  children,
}: {
  id: string
  canvasX: number
  canvasY: number
  width: number
  height: number
  children: React.ReactNode
}) {
  // Shapes have no card background or shadow (transparent). The body
  // children paint the rectangle/ellipse/diamond fill themselves.
  return (
    <div
      data-entity-id={id}
      className="absolute pointer-events-auto"
      style={{
        left: canvasX,
        top: canvasY,
        width,
        height,
        background: 'transparent',
        overflow: 'visible',
        cursor: 'default',
        touchAction: 'none',
      }}
    >
      {children}
    </div>
  )
}

function ShapeCard({
  shape,
  isDark,
  isSelected,
  editing,
  onUpdateText,
  onCommitEdit,
}: {
  shape: CanvasSceneShapeEntity
  isDark: boolean
  isSelected: boolean
  editing: boolean
  onUpdateText: (id: string, text: string) => void
  onCommitEdit: () => void
}) {
  return (
    <ShapeShell
      id={shape.id}
      canvasX={shape.canvasX}
      canvasY={shape.canvasY}
      width={shape.width}
      height={shape.height}
    >
      <MemoShapeBody
        shape={shape}
        isDark={isDark}
        editing={editing}
        selected={isSelected}
        onCommitText={(text) => onUpdateText(shape.id, text)}
        onCommitEdit={onCommitEdit}
      />
    </ShapeShell>
  )
}

export function ShapeBodyLayer({
  entities,
  isDark,
  selectedEntityIdSet,
  editingEntityId,
  canvasOrigin,
  pan,
  zoom,
  onUpdateText,
  onCommitEdit,
}: {
  entities: CanvasSceneShapeEntity[]
  isDark: boolean
  selectedEntityIdSet: Set<string>
  /** id of the entity currently in inline-edit mode (or null). Mounts the
   *  contentEditable iff `editingEntityId === shape.id`. */
  editingEntityId: string | null
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  onUpdateText: (id: string, text: string) => void
  onCommitEdit: () => void
}) {
  if (!entities.length) return null
  return (
    <CanvasViewportLayer canvasOrigin={canvasOrigin} pan={pan} zoom={zoom}>
      {entities.map((shape) => (
        <ShapeCard
          key={shape.id}
          shape={shape}
          isDark={isDark}
          isSelected={selectedEntityIdSet.has(shape.id)}
          editing={editingEntityId === shape.id}
          onUpdateText={onUpdateText}
          onCommitEdit={onCommitEdit}
        />
      ))}
    </CanvasViewportLayer>
  )
}
