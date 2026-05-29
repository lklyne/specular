/**
 * EdgeLayer — node-to-node edge bodies + anchor dots, rendered in aboveView.
 *
 * The svg is purely visual (`pointer-events: none` end-to-end); interaction
 * is driven by `useCanvasPointerRouter` against the layout snapshot. Anchor
 * coords arrive in window-space (`screenX`/`screenY`); aboveView's WCV
 * origin sits at `canvasOrigin.y`, so subtract it from every y when laying
 * out SVG geometry — matching the rest of aboveView.
 */
import { useEffect, useMemo, useState } from 'react'
import type {
  CanvasInteractionState,
  CanvasSceneEntity,
  EdgeEnd,
  EdgeSide,
  WorkspaceEdge,
} from '../../shared/types'
import { resolveCanvasColor } from '../../shared/canvas-colors'
import {
  EDGE_ANCHOR_DOT_OFFSET_PX,
  EDGE_ANCHOR_HIT_ACROSS_PX,
  EDGE_ANCHOR_HIT_ALONG_PX,
  EDGE_ANCHOR_HIT_CORNER_PX,
  EDGE_ANCHOR_HIT_GAP_PX,
  EDGE_SIDES,
} from '../../shared/canvas-hit-geometry'
import { entityHasAnchors } from '../../shared/hit-test'
import { selectionColor, EDGE_COLOR_DEFAULT } from '../canvas-bg/canvasBgConstants'
import { scaleEdgeHitTargetSize } from '../canvas-bg/edgeHitSizing'

// --- Constants ---

const DOT_RADIUS = 3
const CONTROL_POINT_MIN = 40
const CONTROL_POINT_MAX = 200
const EDGE_SELECTION_HIT_WIDTH = 14

// --- Geometry helpers ---

interface AnchorPoint {
  x: number
  y: number
  side: EdgeSide
}

function getAnchorPoint(
  entity: CanvasSceneEntity,
  side: EdgeSide,
  zoom: number,
  originY: number,
): AnchorPoint {
  const { screenX, screenY, screenWidth, screenHeight } = entity
  const localY = screenY - originY
  const dotOffset = EDGE_ANCHOR_DOT_OFFSET_PX * zoom
  switch (side) {
    case 'top':
      return { x: screenX + screenWidth / 2, y: localY - dotOffset, side }
    case 'bottom':
      return { x: screenX + screenWidth / 2, y: localY + screenHeight + dotOffset, side }
    case 'left':
      return { x: screenX - dotOffset, y: localY + screenHeight / 2, side }
    case 'right':
      return { x: screenX + screenWidth + dotOffset, y: localY + screenHeight / 2, side }
  }
}

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

function controlPointOffset(side: EdgeSide, distance: number, zoom: number): { dx: number; dy: number } {
  const offset = Math.min(Math.max(distance * 0.4, CONTROL_POINT_MIN * zoom), CONTROL_POINT_MAX * zoom)
  switch (side) {
    case 'top':
      return { dx: 0, dy: -offset }
    case 'bottom':
      return { dx: 0, dy: offset }
    case 'left':
      return { dx: -offset, dy: 0 }
    case 'right':
      return { dx: offset, dy: 0 }
  }
}

function buildBezierPath(from: AnchorPoint, to: AnchorPoint, zoom: number): string {
  const dist = Math.hypot(to.x - from.x, to.y - from.y)
  const cp1 = controlPointOffset(from.side, dist, zoom)
  const cp2 = controlPointOffset(to.side, dist, zoom)
  return `M ${from.x} ${from.y} C ${from.x + cp1.dx} ${from.y + cp1.dy}, ${to.x + cp2.dx} ${to.y + cp2.dy}, ${to.x} ${to.y}`
}

/** Pick the best sides to connect two entities when sides aren't specified. */
function autoSides(
  from: CanvasSceneEntity,
  to: CanvasSceneEntity,
): { fromSide: EdgeSide; toSide: EdgeSide } {
  const fromCx = from.screenX + from.screenWidth / 2
  const fromCy = from.screenY + from.screenHeight / 2
  const toCx = to.screenX + to.screenWidth / 2
  const toCy = to.screenY + to.screenHeight / 2
  const dx = toCx - fromCx
  const dy = toCy - fromCy
  if (Math.abs(dx) > Math.abs(dy)) {
    return { fromSide: dx > 0 ? 'right' : 'left', toSide: dx > 0 ? 'left' : 'right' }
  }
  return { fromSide: dy > 0 ? 'bottom' : 'top', toSide: dy > 0 ? 'top' : 'bottom' }
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

// --- Main EdgeLayer ---

export function EdgeLayer({
  edges,
  entities,
  hoveredEntityId,
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
  hoveredEntityId: string | null
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
      })
    }
    return paths
  }, [edges, entityMap, selectedEdgeIds, zoom, originY])

  // Which entities should show anchor dots: selected + hovered entities, or all during drag
  const anchorEntities = useMemo(() => {
    const ids = new Set<string>()
    if (selectedEdgeIds.size === 0) {
      for (const id of selectedEntityIds) {
        if (entityMap.has(id)) ids.add(id)
      }
    }
    if (hoveredEntityId && entityMap.has(hoveredEntityId)) ids.add(hoveredEntityId)
    if (interaction.kind === 'dragging-edge') {
      // During drag, show all entity anchors as potential targets
      for (const eId of entityMap.keys()) ids.add(eId)
    }
    return [...ids]
      .map((id) => entityMap.get(id)!)
      .filter((entity) => entity && entityHasAnchors(entity.kind))
  }, [selectedEntityIds, selectedEdgeIds, hoveredEntityId, entityMap, interaction.kind])

  return (
    <svg
      className="pointer-events-none absolute inset-0 h-full w-full"
      style={zIndex === undefined ? undefined : { zIndex }}
    >
      {/* Arrow marker definitions */}
      <defs>
        <marker id="arrow-default" markerHeight={9} markerUnits="userSpaceOnUse" markerWidth={7} orient="auto" refX={7} refY={4.5}>
          <path d="M 0 0 L 7 4.5 L 0 9 Z" fill={EDGE_COLOR_DEFAULT} />
        </marker>
        <marker id="arrow-selected" markerHeight={9} markerUnits="userSpaceOnUse" markerWidth={7} orient="auto" refX={7} refY={4.5}>
          <path d="M 0 0 L 7 4.5 L 0 9 Z" fill={selectionColor(isDark)} />
        </marker>
        <marker id="arrow-start-default" markerHeight={9} markerUnits="userSpaceOnUse" markerWidth={7} orient="auto" refX={0} refY={4.5}>
          <path d="M 7 0 L 0 4.5 L 7 9 Z" fill={EDGE_COLOR_DEFAULT} />
        </marker>
        <marker id="arrow-start-selected" markerHeight={9} markerUnits="userSpaceOnUse" markerWidth={7} orient="auto" refX={0} refY={4.5}>
          <path d="M 7 0 L 0 4.5 L 7 9 Z" fill={selectionColor(isDark)} />
        </marker>
        {/* Per-color markers for colored edges (deduplicated) */}
        {[...new Set(edgePaths.map((p) => p.color).filter(Boolean))].map((color) => {
          const hex = resolveCanvasColor(color!, { palette: 'vivid' })
          const safeId = hex.replace('#', '')
          return (
            <g key={safeId}>
              <marker id={`arrow-color-${safeId}`} markerHeight={9} markerUnits="userSpaceOnUse" markerWidth={7} orient="auto" refX={7} refY={4.5}>
                <path d="M 0 0 L 7 4.5 L 0 9 Z" fill={hex} />
              </marker>
              <marker id={`arrow-start-color-${safeId}`} markerHeight={9} markerUnits="userSpaceOnUse" markerWidth={7} orient="auto" refX={0} refY={4.5}>
                <path d="M 7 0 L 0 4.5 L 7 9 Z" fill={hex} />
              </marker>
            </g>
          )
        })}
      </defs>

      {/* Existing edges */}
      {edgePaths.map(({ id, d, selected, fromEnd, toEnd, color }) => {
        const resolvedColor = color ? resolveCanvasColor(color, { palette: 'vivid' }) : null
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
          <path
            d={d}
            fill="none"
            markerEnd={toEnd === 'arrow' ? `url(#arrow-${markerSuffix})` : undefined}
            markerStart={fromEnd === 'arrow' ? `url(#arrow-start-${markerSuffix})` : undefined}
            stroke={edgeColor}
            strokeWidth={1.5}
          />
          {/* Zoom-scaled invisible hit target. Tagged `data-overlay-ui` so
              the canvas pointer router skips its pointerdown — edge selection
              fires from this path's `onClick` (mirrors main's behavior). */}
          <path
            d={d}
            data-overlay-ui
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
}
