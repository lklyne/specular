import { getStroke } from 'perfect-freehand'
import type {
  AnnotationDrawing,
  AnnotationDrawingStroke,
  CanvasSceneEntity,
  CanvasScenePageEntity,
  LayoutUpdateData,
} from '../../shared/types'
import { PageOverlayBand } from './PageOverlayBand'
import { canvasToScreenX, canvasToScreenY } from '../../shared/gesture-utils'
import {
  paletteForBrushType,
  resolveCanvasColor,
  withAlpha,
} from '../../shared/canvas-colors'

function freehandPathD(
  points: { x: number; y: number }[],
  size: number,
  cap: boolean = true,
): string {
  const outline = getStroke(
    points.map((p) => [p.x, p.y]),
    {
      size: size * 1.6,
      thinning: 0,
      smoothing: 0.8,
      streamline: 0.75,
      simulatePressure: false,
      last: true,
      start: { cap, taper: 0 },
      end: { cap, taper: 0 },
    },
  )
  if (!outline.length) return ''
  const d = outline.reduce(
    (acc, [x, y], i) => acc + (i === 0 ? `M ${x} ${y}` : ` L ${x} ${y}`),
    '',
  )
  return d + ' Z'
}

// Stops applied along the stroke's first→last direction: denser at the ends,
// lighter in the middle, to mimic ink pooling at the start/end of a swipe.
const HIGHLIGHT_ALPHA_STOPS: Array<{ offset: string; alpha: number }> = [
  { offset: '0%', alpha: 0.5 },
  { offset: '4%', alpha: 0.15 },
  { offset: '96%', alpha: 0.3 },
  { offset: '100%', alpha: 0.6 },
]

const GRAIN_FILTER_ID = 'highlight-grain'

function HighlightStroke({
  stroke,
  points,
  visibleWidth,
  active,
  filterId,
  inkColor,
}: {
  stroke: AnnotationDrawingStroke
  points: { x: number; y: number }[]
  visibleWidth: number
  active: boolean
  filterId: string
  inkColor: string
}) {
  if (points.length === 0) return null
  // Render as a perfect-freehand outline filled with the gradient instead of a
  // stroked polyline. The streamline/smoothing params absorb pointer jitter at
  // pointerdown/pointerup, which would otherwise show up as flag-shaped
  // artifacts at the start/end of strokes (the butt cap is perpendicular to
  // the first/last segment direction, so any micro-diagonal jitter there
  // becomes a visible angled tail).
  const filledD = freehandPathD(points, visibleWidth, false)
  if (!filledD) return null

  const first = points[0]
  const last = points[points.length - 1]
  const dx = last.x - first.x
  const dy = last.y - first.y
  const len = Math.hypot(dx, dy)
  const x1 = first.x
  const y1 = first.y
  const x2 = len < 1 ? first.x + 1 : last.x
  const y2 = len < 1 ? first.y : last.y
  const gradientId = `hl-grad-${stroke.id}`
  return (
    <>
      <defs>
        <linearGradient
          id={gradientId}
          gradientUnits="userSpaceOnUse"
          x1={x1}
          y1={y1}
          x2={x2}
          y2={y2}
        >
          {HIGHLIGHT_ALPHA_STOPS.map((s) => (
            <stop key={s.offset} offset={s.offset} stopColor={withAlpha(inkColor, s.alpha)} />
          ))}
        </linearGradient>
      </defs>
      <path
        d={filledD}
        fill={`url(#${gradientId})`}
        filter={`url(#${filterId})`}
        opacity={active ? 1 : 0.95}
      />
    </>
  )
}

function renderStrokeBody({
  stroke,
  points,
  visibleWidth,
  active,
  isDark,
}: {
  stroke: AnnotationDrawingStroke
  points: { x: number; y: number }[]
  visibleWidth: number
  active: boolean
  isDark: boolean
}) {
  const inkColor = resolveCanvasColor(stroke.color, {
    role: 'ink',
    isDark,
    palette: paletteForBrushType(stroke.brushType ?? 'pen'),
  })
  if (stroke.brushType === 'highlight') {
    return (
      <HighlightStroke
        stroke={stroke}
        points={points}
        visibleWidth={visibleWidth}
        active={active}
        filterId={GRAIN_FILTER_ID}
        inkColor={inkColor}
      />
    )
  }
  // Pen strokes render fully opaque — only the highlighter is translucent.
  return <path d={freehandPathD(points, visibleWidth)} fill={inkColor} />
}

export function DrawingLayer({
  drawing,
  layout,
  active,
  isDark,
}: {
  drawing: AnnotationDrawing
  layout: LayoutUpdateData
  active?: boolean
  isDark: boolean
}) {
  const hasHighlight = drawing.strokes.some((s) => s.brushType === 'highlight')
  return (
    <svg
      className="pointer-events-none absolute inset-0"
      width={window.innerWidth}
      height={window.innerHeight}
      viewBox={`0 0 ${window.innerWidth} ${window.innerHeight}`}
      aria-hidden="true"
    >
      {hasHighlight ? (
        <defs>
          {/*
            filterUnits="userSpaceOnUse" is load-bearing: the default
            ("objectBoundingBox") sizes the filter region as a percentage of the
            path's geometric bbox, which does NOT include strokeWidth. A purely
            horizontal stroke has bbox height = 0 → filter region height = 0 →
            the whole 22px-thick stroke gets clipped to nothing. With user
            space, the region is the SVG viewport so any stroke geometry works.
          */}
          <filter
            id={GRAIN_FILTER_ID}
            x="0"
            y="0"
            width={window.innerWidth}
            height={window.innerHeight}
            filterUnits="userSpaceOnUse"
            primitiveUnits="userSpaceOnUse"
          >
            <feTurbulence
              type="fractalNoise"
              baseFrequency="0.85"
              numOctaves={2}
              seed={4}
              stitchTiles="stitch"
              result="noise"
            />
            <feColorMatrix
              in="noise"
              type="matrix"
              values="0 0 0 0 0
                      0 0 0 0 0
                      0 0 0 0 0
                      0.35 0 0 0 0.78"
              result="noiseAlpha"
            />
            <feComposite in="SourceGraphic" in2="noiseAlpha" operator="in" />
          </filter>
        </defs>
      ) : null}
      {drawing.strokes.map((stroke) => {
        const points = stroke.points.map((point) => ({
          x: canvasToScreenX(layout, point.x),
          y: canvasToScreenY(layout, point.y) - layout.canvasOrigin.y,
        }))
        const visibleWidth = Math.max(1, stroke.width * layout.zoom)
        return (
          <g key={stroke.id}>
            {renderStrokeBody({ stroke, points, visibleWidth, active: active ?? false, isDark })}
          </g>
        )
      })}
    </svg>
  )
}

export function SavedDrawingEntities({
  entities,
  layoutData,
  selectedEntityIds,
  isDark,
}: {
  entities: CanvasSceneEntity[]
  layoutData: LayoutUpdateData
  selectedEntityIds: string[]
  isDark: boolean
}) {
  const drawings = entities.filter(
    (e): e is import('../../shared/types').CanvasSceneDrawingEntity => e.kind === 'drawing',
  )
  if (drawings.length === 0) return null

  return (
    <>
      {drawings.map((drawing) => {
        const isSelected = selectedEntityIds.includes(drawing.id)
        const layer = (
          <DrawingLayer
            key={drawing.id}
            drawing={{ version: 1, bounds: { x: drawing.canvasX, y: drawing.canvasY, width: drawing.width, height: drawing.height }, strokes: drawing.strokes }}
            layout={layoutData}
            active={isSelected}
            isDark={isDark}
          />
        )
        // A page-anchored drawing scroll-follows its page (main shifts the
        // scene strokes), so it clips and edge-fades inside the page's
        // overlay band like shapes do.
        const anchorPageId = drawing.pageAnchor?.pageId
        const page = anchorPageId
          ? layoutData.entities.find(
              (entity): entity is CanvasScenePageEntity =>
                entity.kind === 'page' && entity.id === anchorPageId,
            )
          : undefined
        if (!page) return layer
        return (
          <PageOverlayBand key={drawing.id} page={page} layoutData={layoutData} followScroll>
            {layer}
          </PageOverlayBand>
        )
      })}
    </>
  )
}
