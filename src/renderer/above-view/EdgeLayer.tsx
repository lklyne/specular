/**
 * EdgeLayer — node-to-node edge bodies + anchor dots, rendered in aboveView.
 *
 * The svg is purely visual (`pointer-events: none` end-to-end); interaction
 * is driven by `useCanvasPointerRouter` against the layout snapshot. Anchor
 * coords arrive in window-space (`screenX`/`screenY`); aboveView's WCV
 * origin sits at `canvasOrigin.y`, so subtract it from every y when laying
 * out SVG geometry — matching the rest of aboveView.
 */
import { memo, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react'
import type {
  CanvasInteractionState,
  CanvasSceneEntity,
  EdgeEnd,
  EdgeLineStyle,
  EdgeSide,
  WorkspaceEdge,
} from '../../shared/types'
import { resolveCanvasColor } from '../../shared/canvas-colors'
import {
  EDGE_ANCHOR_HIT_ACROSS_PX,
  EDGE_ANCHOR_HIT_ALONG_PX,
  EDGE_ANCHOR_HIT_CORNER_PX,
  EDGE_ANCHOR_HIT_GAP_PX,
  EDGE_SIDES,
} from '../../shared/canvas-hit-geometry'
import { anchorEligibleEntityIds, entityHasAnchors } from '../../shared/hit-test'
import { useHoveredEntityId } from '../shared/hooks/useHoveredEntityId'
import {
  autoSides,
  buildBezierPath,
  getAnchorPoint,
  type AnchorPoint,
} from '../../shared/edge-geometry'
import { selectionColor, EDGE_COLOR_DEFAULT } from '../canvas-bg/canvasBgConstants'
import { scaleEdgeHitTargetSize } from '../canvas-bg/edgeHitSizing'

// --- Constants ---

const DOT_RADIUS = 4
const EDGE_SELECTION_HIT_WIDTH = 14
const LABEL_FONT_SIZE = 16 // canvas units, scaled by zoom
const LABEL_GAP_PAD_X = 6 // canvas units of horizontal clearance on each side of the text
const LABEL_GAP_PAD_Y = 0 // canvas units of vertical clearance above/below the text
// Pull arrowheads slightly past the path endpoints so thick strokes terminate
// underneath the triangle instead of leaving a square tip visible beyond it.
const END_ARROW_REF_X = 4.25
const START_ARROW_REF_X = 0.75

// --- Geometry helpers ---

function getAnchorHitRect(
  entity: CanvasSceneEntity,
  side: EdgeSide,
  zoom: number,
  originY: number,
): { x: number; y: number; width: number; height: number } {
  const { screenX, screenY, screenWidth, screenHeight } = entity
  const localY = screenY - originY
  const along = scaleEdgeHitTargetSize(EDGE_ANCHOR_HIT_ALONG_PX, zoom)
  const across = scaleEdgeHitTargetSize(EDGE_ANCHOR_HIT_ACROSS_PX, zoom)
  const horizontal = side === 'top' || side === 'bottom'
  const width = horizontal ? along : across
  const height = horizontal ? across : along
  const cx = screenX + screenWidth / 2
  const cy = localY + screenHeight / 2
  switch (side) {
    case 'top':
      return { x: cx - width / 2, y: localY - EDGE_ANCHOR_HIT_GAP_PX - height, width, height }
    case 'bottom':
      return { x: cx - width / 2, y: localY + screenHeight + EDGE_ANCHOR_HIT_GAP_PX, width, height }
    case 'left':
      return { x: screenX - EDGE_ANCHOR_HIT_GAP_PX - width, y: cy - height / 2, width, height }
    case 'right':
      return { x: screenX + screenWidth + EDGE_ANCHOR_HIT_GAP_PX, y: cy - height / 2, width, height }
  }
}

// --- Anchor dots for a single entity ---

function AnchorDots({
  entity,
  isDark,
  isDragging,
  zoom,
  originY,
}: {
  entity: CanvasSceneEntity
  isDark: boolean
  isDragging: boolean
  zoom: number
  originY: number
}) {
  const [hoveredSide, setHoveredSide] = useState<EdgeSide | null>(null)

  useEffect(() => {
    if (!isDragging) setHoveredSide(null)
  }, [isDragging])

  return (
    <>
      {EDGE_SIDES.map((side) => {
        const pt = getAnchorPoint(entity, side, zoom, originY)
        const hitRect = getAnchorHitRect(entity, side, zoom, originY)
        const showDot = isDragging || hoveredSide === side
        return (
          <g key={side}>
            {showDot ? (
              <circle
                cx={pt.x}
                cy={pt.y}
                fill="white"
                r={DOT_RADIUS}
                stroke={selectionColor(isDark)}
                strokeWidth={1}
              />
            ) : null}
            {/* Hit rect drives per-side hover so the dot lights up before drag.
                Pointer-down still routes through `useCanvasPointerRouter`'s
                window listener — we just need pointer-events on for hover and
                cursor styling. */}
            <rect
              x={hitRect.x}
              y={hitRect.y}
              width={hitRect.width}
              height={hitRect.height}
              rx={EDGE_ANCHOR_HIT_CORNER_PX}
              ry={EDGE_ANCHOR_HIT_CORNER_PX}
              fill="transparent"
              style={{ cursor: 'crosshair', pointerEvents: 'all' }}
              onPointerEnter={() => setHoveredSide(side)}
              onPointerLeave={() => {
                if (isDragging) return
                setHoveredSide((current) => (current === side ? null : current))
              }}
            />
          </g>
        )
      })}
    </>
  )
}

// --- Edge body: the visible stroke, plus a centered label with a gap ---
//
// The label sits at the path's arc-length midpoint. We knock a gap into the
// stroke instead of painting a background rect behind the text — a rect would
// cover the dotted grid, whereas a dash gap leaves it showing. The gap is
// centered via `stroke-dasharray: dash gap` where dash = (len - gap) / 2, so
// the pattern draws dash, gap, dash and lands exactly at the path end.

function EdgeBody({
  d,
  edgeColor,
  labelColor,
  label,
  zoom,
  markerEnd,
  markerStart,
  strokeWidth,
  lineStyle,
}: {
  d: string
  edgeColor: string
  labelColor: string
  label: string | undefined
  zoom: number
  markerEnd: string | undefined
  markerStart: string | undefined
  strokeWidth: number
  lineStyle: EdgeLineStyle
}) {
  const pathRef = useRef<SVGPathElement>(null)
  const textRef = useRef<SVGTextElement>(null)
  const [layout, setLayout] = useState<{ mx: number; my: number; dash: string | undefined } | null>(null)

  useLayoutEffect(() => {
    if (!label) {
      setLayout(null)
      return
    }
    const path = pathRef.current
    const text = textRef.current
    if (!path || !text) return
    const len = path.getTotalLength()
    const mid = path.getPointAtLength(len / 2)
    // Line direction at the midpoint, to size the gap by the text box projected
    // onto the line rather than always by its width — a steep crossing then
    // clears by the text's height, not its (much larger) width.
    const a = path.getPointAtLength(Math.max(0, len / 2 - 1))
    const b = path.getPointAtLength(Math.min(len, len / 2 + 1))
    const angle = Math.atan2(b.y - a.y, b.x - a.x)
    const box = text.getBBox()
    const gapW = box.width + LABEL_GAP_PAD_X * 2 * zoom
    const gapH = box.height + LABEL_GAP_PAD_Y * 2 * zoom
    const gap = Math.abs(gapW * Math.cos(angle)) + Math.abs(gapH * Math.sin(angle))
    // Short edge: skip the gap rather than emit a negative dash.
    const dash = gap < len ? `${(len - gap) / 2} ${gap}` : undefined
    setLayout({ mx: mid.x, my: mid.y, dash })
  }, [d, label, zoom])

  return (
    <>
      <path
        ref={pathRef}
        d={d}
        fill="none"
        markerEnd={markerEnd}
        markerStart={markerStart}
        stroke={edgeColor}
        strokeWidth={strokeWidth * zoom}
        strokeDasharray={
          label
            ? layout?.dash
            : lineStyle === 'dashed'
              ? `${strokeWidth * 3 * zoom} ${strokeWidth * 2 * zoom}`
              : undefined
        }
      />
      {label ? (
        <text
          ref={textRef}
          x={layout?.mx ?? 0}
          y={layout?.my ?? 0}
          fill={labelColor}
          fontFamily="system-ui, sans-serif"
          fontSize={LABEL_FONT_SIZE * zoom}
          textAnchor="middle"
          dominantBaseline="central"
          // Hidden until measured so it doesn't flash at 0,0 on first paint.
          visibility={layout ? 'visible' : 'hidden'}
        >
          {label}
        </text>
      ) : null}
    </>
  )
}

// --- Main EdgeLayer ---

export const EdgeLayer = memo(function EdgeLayer({
  edges,
  entities,
  isDark,
  interaction,
  selectedEdgeIds,
  selectedEntityIds,
  zoom,
  originY,
  onSelectEdge,
  renderAnchors = true,
  zIndex = 5,
}: {
  edges: WorkspaceEdge[]
  entities: CanvasSceneEntity[]
  isDark: boolean
  interaction: CanvasInteractionState
  selectedEdgeIds: ReadonlySet<string>
  selectedEntityIds: string[]
  zoom: number
  originY: number
  onSelectEdge: (edgeId: string) => void
  renderAnchors?: boolean
  zIndex?: number | undefined
}) {
  const entityMap = useMemo(() => {
    const map = new Map<string, CanvasSceneEntity>()
    for (const e of entities) map.set(e.id, e)
    return map
  }, [entities])

  const edgeSelectionHitWidth = scaleEdgeHitTargetSize(EDGE_SELECTION_HIT_WIDTH, zoom)

  // Render existing edges (skip the one being re-routed — the dashed drag
  // path stands in for it).
  const edgePaths = useMemo(() => {
    const paths: Array<{
      id: string
      d: string
      selected: boolean
      fromEnd: EdgeEnd
      toEnd: EdgeEnd
      color?: string
      label?: string
      strokeWidth: number
      lineStyle: EdgeLineStyle
    }> = []

    for (const edge of edges) {
      const fromEntity = entityMap.get(edge.fromEntityId)
      const toEntity = entityMap.get(edge.toEntityId)
      if (!fromEntity || !toEntity) continue

      const { fromSide, toSide } = edge.fromSide && edge.toSide
        ? { fromSide: edge.fromSide, toSide: edge.toSide }
        : autoSides(fromEntity, toEntity)

      const from = getAnchorPoint(fromEntity, fromSide, zoom, originY)
      const to = getAnchorPoint(toEntity, toSide, zoom, originY)
      const d = buildBezierPath(from, to, zoom)
      paths.push({
        id: edge.id,
        d,
        selected: selectedEdgeIds.has(edge.id),
        fromEnd: edge.fromEnd ?? 'none',
        toEnd: edge.toEnd ?? 'arrow',
        color: edge.color,
        label: edge.label?.trim() || undefined,
        strokeWidth: edge.strokeWidth ?? 1.5,
        lineStyle: edge.lineStyle ?? 'solid',
      })
    }
    return paths
  }, [edges, entityMap, selectedEdgeIds, zoom, originY])

  // Hovering a node makes its anchors grabbable, so this layer subscribes to
  // hover directly instead of taking it off the layout snapshot.
  const hoveredEntityId = useHoveredEntityId()

  // Which entities show anchor dots: the shared eligibility selector (kept in
  // lockstep with the hit-tester's `collectAnchorTargets`), plus every entity
  // while an edge drag is live — all anchors are potential drop targets then.
  const anchorEntities = useMemo(() => {
    const ids = anchorEligibleEntityIds({
      selectedEntityIds,
      hoveredEntityId,
      edgeSelected: selectedEdgeIds.size > 0,
    })
    if (interaction.kind === 'dragging-edge') {
      for (const eId of entityMap.keys()) ids.add(eId)
    }
    return [...ids]
      .map((id) => entityMap.get(id))
      .filter((entity): entity is NonNullable<typeof entity> =>
        !!entity && entityHasAnchors(entity.kind),
      )
  }, [selectedEntityIds, selectedEdgeIds, hoveredEntityId, entityMap, interaction.kind])

  return (
    <svg
      className="pointer-events-none absolute inset-0 h-full w-full"
      style={zIndex === undefined ? undefined : { zIndex }}
    >
      {/* Arrow marker definitions */}
      <defs>
        <marker id="arrow-default" markerHeight={6} markerWidth={5} orient="auto" refX={END_ARROW_REF_X} refY={3} overflow="visible">
          <path d="M 0 0 L 5 3 L 0 6 Z" fill={EDGE_COLOR_DEFAULT} />
        </marker>
        <marker id="arrow-selected" markerHeight={6} markerWidth={5} orient="auto" refX={END_ARROW_REF_X} refY={3} overflow="visible">
          <path d="M 0 0 L 5 3 L 0 6 Z" fill={selectionColor(isDark)} />
        </marker>
        <marker id="arrow-start-default" markerHeight={6} markerWidth={5} orient="auto" refX={START_ARROW_REF_X} refY={3} overflow="visible">
          <path d="M 5 0 L 0 3 L 5 6 Z" fill={EDGE_COLOR_DEFAULT} />
        </marker>
        <marker id="arrow-start-selected" markerHeight={6} markerWidth={5} orient="auto" refX={START_ARROW_REF_X} refY={3} overflow="visible">
          <path d="M 5 0 L 0 3 L 5 6 Z" fill={selectionColor(isDark)} />
        </marker>
        {/* Per-color markers for colored edges (deduplicated) */}
        {[...new Set(edgePaths.map((p) => p.color).filter(Boolean))].map((color) => {
          const hex = resolveCanvasColor(color!, {
            palette: 'vivid',
            role: 'ink',
            isDark,
          })
          const safeId = hex.replace('#', '')
          return (
            <g key={safeId}>
              <marker id={`arrow-color-${safeId}`} markerHeight={6} markerWidth={5} orient="auto" refX={END_ARROW_REF_X} refY={3} overflow="visible">
                <path d="M 0 0 L 5 3 L 0 6 Z" fill={hex} />
              </marker>
              <marker id={`arrow-start-color-${safeId}`} markerHeight={6} markerWidth={5} orient="auto" refX={START_ARROW_REF_X} refY={3} overflow="visible">
                <path d="M 5 0 L 0 3 L 5 6 Z" fill={hex} />
              </marker>
            </g>
          )
        })}
      </defs>

      {/* Existing edges */}
      {edgePaths.map(({ id, d, selected, fromEnd, toEnd, color, label, strokeWidth, lineStyle }) => {
        const resolvedColor = color
          ? resolveCanvasColor(color, { palette: 'vivid', role: 'ink', isDark })
          : null
        const edgeColor = selected
          ? selectionColor(isDark)
          : resolvedColor ?? EDGE_COLOR_DEFAULT
        const markerSuffix = selected
          ? 'selected'
          : resolvedColor
            ? `color-${resolvedColor.replace('#', '')}`
            : 'default'
        return (
        <g key={id}>
          <EdgeBody
            d={d}
            edgeColor={edgeColor}
            labelColor={isDark ? '#e7e5e4' : '#1c1917'}
            label={label}
            zoom={zoom}
            markerEnd={toEnd === 'arrow' ? `url(#arrow-${markerSuffix})` : undefined}
            markerStart={fromEnd === 'arrow' ? `url(#arrow-start-${markerSuffix})` : undefined}
            strokeWidth={strokeWidth}
            lineStyle={lineStyle}
          />
          {/* Zoom-scaled invisible hit target. Tagged `data-overlay-ui` so
              the canvas pointer router skips its pointerdown — edge selection
              fires from this path's `onClick` (mirrors main's behavior). */}
          <path
            d={d}
            data-overlay-ui
            data-edge-id={id}
            fill="none"
            stroke="transparent"
            strokeWidth={edgeSelectionHitWidth}
            style={{ cursor: 'pointer', pointerEvents: 'stroke' }}
            onClick={(e) => {
              e.stopPropagation()
              onSelectEdge(id)
            }}
          />
        </g>
        )
      })}

      {/* Anchor dots */}
      {renderAnchors ? anchorEntities.map((entity) => (
        <AnchorDots
          key={entity.id}
          entity={entity}
          isDark={isDark}
          isDragging={interaction.kind === 'dragging-edge'}
          zoom={zoom}
          originY={originY}
        />
      )) : null}
    </svg>
  )
})
