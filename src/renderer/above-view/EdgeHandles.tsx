/**
 * Handles on a selected edge — endpoint dots and the elbow crossbar grip.
 *
 * These follow the DOM pattern the rest of the edge already uses (an SVG node
 * with `pointerEvents`, `data-overlay-ui` so the router yields) rather than
 * entering the shared hit-test, which edge bodies also stay out of.
 *
 * Endpoint dots are an affordance, not a separate gesture: a bound endpoint
 * sits exactly on its entity's anchor, whose hit rect the router already
 * classifies as an edge drag and the controller already resolves to the
 * re-route branch. Drawing them makes the existing door visible on the state
 * that implies it. A free endpoint has no anchor rect behind it, so it reads
 * as a marker only.
 *
 * The crossbar grip is a real gesture: it drives `routing-edge` through main
 * (start → move* → commit | cancel), so the live split rides the broadcast and
 * exactly one doc write lands at commit.
 */

import { useRef } from 'react'
import type { EdgeSplitAxis } from '../../shared/types'
import type { CanvasBgElectronAPI } from '../../shared/electron-api/canvas-bg'
import {
  buildElbowPoints,
  elbowAdjustableAxis,
  type AnchorPoint,
  type GeometryPoint,
} from '../../shared/edge-geometry'
import { selectionColor } from '../canvas-bg/canvasBgConstants'

export type EdgeRoutingApi = Pick<
  CanvasBgElectronAPI,
  'beginEdgeRouting' | 'edgeRoutingMove' | 'edgeRoutingCommit' | 'edgeRoutingCancel'
>

const ENDPOINT_RADIUS = 4.5
const GRIP_LENGTH = 18
const GRIP_THICKNESS = 10

/** The crossbar of an elbow route, or null when the route has nothing to drag
 *  (an L is fully determined by its endpoints; an S ships non-adjustable). */
export function elbowCrossbar(
  from: AnchorPoint,
  to: AnchorPoint,
  zoom: number,
  split: { value: number; axis: EdgeSplitAxis } | undefined,
): { a: GeometryPoint; b: GeometryPoint; axis: EdgeSplitAxis; ratio: number; span: number } | null {
  const axis = elbowAdjustableAxis(from, to)
  if (!axis) return null
  const points = buildElbowPoints(from, to, zoom, split)
  if (points.length < 4) return null
  const span = axis === 'x' ? to.x - from.x : to.y - from.y
  if (span === 0) return null
  const a = points[1]
  const b = points[2]
  const ratio = ((axis === 'x' ? a.x : a.y) - (axis === 'x' ? from.x : from.y)) / span
  return { a, b, axis, ratio, span }
}

export function EdgeHandlesLayer({
  edgeId,
  from,
  to,
  zoom,
  isDark,
  routing,
  split,
  api,
}: {
  edgeId: string
  from: AnchorPoint
  to: AnchorPoint
  zoom: number
  isDark: boolean
  routing: string | undefined
  split: { value: number; axis: EdgeSplitAxis } | undefined
  api?: EdgeRoutingApi
}) {
  const color = selectionColor(isDark)
  const crossbar = routing === 'elbow' ? elbowCrossbar(from, to, zoom, split) : null
  return (
    <g>
      {[from, to].map((point, index) => (
        <circle
          key={index === 0 ? 'from' : 'to'}
          cx={point.x}
          cy={point.y}
          r={ENDPOINT_RADIUS}
          fill="white"
          stroke={color}
          strokeWidth={1.5}
        />
      ))}
      {crossbar && api ? (
        <SegmentGrip
          edgeId={edgeId}
          crossbar={crossbar}
          color={color}
          api={api}
        />
      ) : null}
    </g>
  )
}

function SegmentGrip({
  edgeId,
  crossbar,
  color,
  api,
}: {
  edgeId: string
  crossbar: NonNullable<ReturnType<typeof elbowCrossbar>>
  color: string
  api: EdgeRoutingApi
}) {
  const drag = useRef<{ startClient: number; startRatio: number } | null>(null)
  const { a, b, axis, ratio, span } = crossbar
  const cx = (a.x + b.x) / 2
  const cy = (a.y + b.y) / 2
  // The grip lies across the crossbar: a crossbar adjustable on x is a
  // vertical bar the user slides horizontally.
  const vertical = axis === 'x'
  const width = vertical ? GRIP_THICKNESS : GRIP_LENGTH
  const height = vertical ? GRIP_LENGTH : GRIP_THICKNESS

  const onPointerMove = (event: PointerEvent) => {
    const state = drag.current
    if (!state) return
    const client = axis === 'x' ? event.clientX : event.clientY
    // Deltas are identical in client and overlay space, so no origin math.
    const next = state.startRatio + (client - state.startClient) / span
    api.edgeRoutingMove(Math.min(1, Math.max(0, next)))
  }

  const end = (commit: boolean) => {
    drag.current = null
    window.removeEventListener('pointermove', onPointerMove)
    window.removeEventListener('pointerup', onUp)
    window.removeEventListener('pointercancel', onCancel)
    if (commit) api.edgeRoutingCommit()
    else api.edgeRoutingCancel('escape')
  }
  const onUp = () => end(true)
  const onCancel = () => end(false)

  return (
    <rect
      data-overlay-ui
      x={cx - width / 2}
      y={cy - height / 2}
      width={width}
      height={height}
      rx={GRIP_THICKNESS / 2}
      fill={color}
      fillOpacity={0.001}
      stroke={color}
      strokeOpacity={0.9}
      strokeWidth={2}
      style={{ cursor: vertical ? 'col-resize' : 'row-resize', pointerEvents: 'all' }}
      onPointerDown={(event) => {
        event.stopPropagation()
        event.preventDefault()
        drag.current = {
          startClient: axis === 'x' ? event.clientX : event.clientY,
          startRatio: ratio,
        }
        api.beginEdgeRouting(edgeId, ratio, axis)
        window.addEventListener('pointermove', onPointerMove)
        window.addEventListener('pointerup', onUp)
        window.addEventListener('pointercancel', onCancel)
      }}
    />
  )
}
