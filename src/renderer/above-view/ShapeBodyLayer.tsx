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
import type { CanvasSceneShapeEntity, LayoutUpdateData } from '../../shared/types'
import { darkenHex, lightenHex, resolveCanvasColor } from '../../shared/canvas-colors'
import { shapeDef } from '../../shared/shapes'
import { CanvasViewportLayer, EntityShell } from './CanvasViewportLayer'
import { AnchoredEntityOverlayBand } from './PageOverlayBand'

const DEFAULT_STROKE_WIDTH = 2
/** ADR 0013 §2 — shapes without textSize render their label at this size. */
const DEFAULT_TEXT_SIZE = 14
// Light mode lightens the hue toward paper; dark mode darkens it toward the
// canvas so the shape reads as a tinted panel, not a bright blob (the pastel
// hue itself stays the border, outlining the dark fill). The palette hues carry
// no dark variant, so the theme split lives here in the derivation.
const FILL_LIGHTEN = 0.5
const FILL_DARKEN = 0.55
// Light-mode borders darken the hue so the outline reads against the light
// canvas — neutral's near-white fill hue would otherwise vanish into it.
const BORDER_DARKEN_LIGHT = 0.35
const NEUTRAL_SLATE = '#6b7280'

// contentEditable represents line breaks as <br>/block elements, which
// .textContent drops entirely. innerText is layout-aware and reconstructs
// `\n` from them, but browsers also report a trailing `\n` for the final
// (empty) line — strip that one so it doesn't accumulate across edits.
function readInnerText(node: HTMLDivElement): string {
  return node.innerText.replace(/\n$/, '')
}

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
        onInput={(e) => onChange(readInnerText(e.target as HTMLDivElement))}
        onPointerDown={(e) => { if (editing) e.stopPropagation() }}
        onBlur={(e) => {
          onCommit(readInnerText(e.target as HTMLDivElement))
        }}
        onKeyDown={(e) => {
          if (e.key === 'Escape') {
            e.preventDefault()
            onCommit(readInnerText(e.target as HTMLDivElement))
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

// Pure fill/border/text derivation for a shape — kept out of the component so
// its render stays about state and layout, not color math.
function shapeVisuals(shape: CanvasSceneShapeEntity, isDark: boolean, editing: boolean) {
  const stroke = shape.strokeWidth ?? DEFAULT_STROKE_WIDTH
  const borderStyle = shape.borderStyle ?? 'solid'
  const hasBorder = borderStyle !== 'none'
  const resolvedColor = shape.color
    ? resolveCanvasColor(shape.color, { role: 'fill', isDark, palette: 'soft' })
    : NEUTRAL_SLATE
  // Opaque fill — the resolved hue pushed toward paper (light) or the canvas
  // (dark), no alpha.
  const fill = isDark
    ? darkenHex(resolvedColor, FILL_DARKEN)
    : lightenHex(resolvedColor, FILL_LIGHTEN)
  // Border color is independent of fill; absent, it derives from the fill hue.
  const borderBase = shape.borderColor
    ? resolveCanvasColor(shape.borderColor, { role: 'fill', isDark, palette: 'soft' })
    : resolvedColor
  // Dark mode: the pastel hue is already a light outline on the dark fill.
  // Light mode: darken it so the edge reads against the light canvas.
  const strokeColor = isDark ? borderBase : darkenHex(borderBase, BORDER_DARKEN_LIGHT)
  const dash = borderStyle === 'dashed' ? `${stroke * 2} ${stroke * 1.5}` : undefined
  const textColor = isDark ? 'rgb(220, 220, 220)' : 'rgb(20, 20, 20)'

  const def = shapeDef(shape.shapeKind)
  const inset = def.textInset
  const textContainerStyle: React.CSSProperties = {
    position: 'absolute',
    left: inset ? `${inset.x}%` : 0,
    top: inset ? `${inset.y}%` : 0,
    ...(inset ? { width: `${inset.w}%`, height: `${inset.h}%` } : { right: 0, bottom: 0 }),
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 8,
    boxSizing: 'border-box',
    pointerEvents: editing ? 'auto' : 'none',
  }
  return { def, fill, hasBorder, stroke, strokeColor, dash, textColor, textContainerStyle }
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

  const { def, fill, hasBorder, stroke, strokeColor, dash, textColor, textContainerStyle } =
    shapeVisuals(shape, isDark, editing)

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

  // Every shape is one SVG path in a normalized 0–100 box, stretched to the
  // entity's bounds. `non-scaling-stroke` keeps the border uniform even when
  // the box is non-square; `strokeDasharray` reproduces the dashed style SVG
  // has no `border-style` for.
  return (
    <>
      <svg
        width="100%"
        height="100%"
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
        style={{ position: 'absolute', inset: 0, pointerEvents: 'none', overflow: 'visible' }}
      >
        <path
          d={def.path}
          fill={fill}
          stroke={hasBorder ? strokeColor : 'none'}
          strokeWidth={hasBorder ? stroke : 0}
          strokeDasharray={dash}
          strokeLinejoin="round"
          vectorEffect="non-scaling-stroke"
        />
        {def.line ? (
          <path
            d={def.line}
            fill="none"
            stroke={hasBorder ? strokeColor : fill}
            strokeWidth={hasBorder ? stroke : 1}
            strokeDasharray={dash}
            vectorEffect="non-scaling-stroke"
          />
        ) : null}
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
    a.shape.borderStyle === b.shape.borderStyle &&
    a.shape.borderColor === b.shape.borderColor &&
    a.shape.textSize === b.shape.textSize &&
    a.shape.width === b.shape.width &&
    a.shape.height === b.shape.height &&
    a.isDark === b.isDark &&
    a.editing === b.editing &&
    a.selected === b.selected
  )
})

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
  // Shapes have no card background or shadow (transparent). The body
  // children paint the rectangle/ellipse/diamond fill themselves.
  return (
    <EntityShell
      id={shape.id}
      canvasX={shape.canvasX}
      canvasY={shape.canvasY}
      style={{
        width: shape.width,
        height: shape.height,
        background: 'transparent',
        overflow: 'visible',
      }}
    >
      <MemoShapeBody
        shape={shape}
        isDark={isDark}
        editing={editing}
        selected={isSelected}
        onCommitText={(text) => onUpdateText(shape.id, text)}
        onCommitEdit={onCommitEdit}
      />
    </EntityShell>
  )
}

export function ShapeBodyLayer({
  entities,
  isDark,
  selectedEntityIdSet,
  editingEntityId,
  layoutData,
  onUpdateText,
  onCommitEdit,
}: {
  entities: CanvasSceneShapeEntity[]
  isDark: boolean
  selectedEntityIdSet: Set<string>
  /** id of the entity currently in inline-edit mode (or null). Mounts the
   *  contentEditable iff `editingEntityId === shape.id`. */
  editingEntityId: string | null
  layoutData: LayoutUpdateData
  onUpdateText: (id: string, text: string) => void
  onCommitEdit: () => void
}) {
  if (!entities.length) return null
  const viewport = (
    <CanvasViewportLayer
      canvasOrigin={layoutData.canvasOrigin}
      pan={layoutData.pan}
      zoom={layoutData.zoom}
    >
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
  // A page-anchored shape scroll-follows its page (main shifts the scene
  // coords), so it clips and edge-fades inside the page's overlay band like
  // annotations do. App mounts one layer per entity, so the single entity's
  // anchor decides the wrapping.
  const anchor = entities[0].pageAnchor
  return (
    <AnchoredEntityOverlayBand anchor={anchor} layoutData={layoutData}>
      {viewport}
    </AnchoredEntityOverlayBand>
  )
}
