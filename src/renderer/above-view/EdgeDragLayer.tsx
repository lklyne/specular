/**
 * Renders the rubber-band line for an in-progress edge drag.
 *
 * Phase 2 of ADR 0001 — the visual used to live in `EdgeLayer.tsx` (bgView)
 * and was driven by EdgeLayer's local React state. With the canvas pointer
 * router taking over pointerdown in aboveView, the visual moves here so the
 * router and the rendering layer share one source of truth: the
 * `EdgeDragState` produced by `src/shared/edge-drag-controller.ts`.
 *
 * Coordinates are window-relative (matches the controller's screen-space
 * geometry). aboveView's WCV starts at `canvasOrigin.y`, so the SVG is
 * translated up by that amount to meet window-space.
 */

import { useMemo } from 'react'
import {
  buildEdgeDragPath,
  type EdgeDragState,
} from '../../shared/edge-drag-controller'
import type { CanvasSceneEntity, LayoutUpdateData } from '../../shared/types'
import { selectionColor } from '../canvas-bg/canvasBgConstants'

const PATH_STROKE_WIDTH = 2
const PATH_DASH = '6 4'

export function EdgeDragLayer({
  state,
  layoutData,
  isDark,
}: {
  state: EdgeDragState
  layoutData: LayoutUpdateData
  isDark: boolean
}) {
  const entityMap = useMemo(() => {
    const map = new Map<string, CanvasSceneEntity>()
    for (const e of layoutData.entities) map.set(e.id, e)
    return map
  }, [layoutData.entities])

  // layoutData is structurally a Camera (pan/zoom/canvasOrigin). The camera is
  // static during an edge drag, so the payload camera equals the live camera and
  // the reprojected container renders the path in place.
  const path = useMemo(
    () => buildEdgeDragPath(state, entityMap, layoutData),
    [state, entityMap, layoutData],
  )

  if (!path) return null

  const color = selectionColor(isDark)
  const offsetY = layoutData.canvasOrigin.y

  return (
    <svg
      data-overlay-ui
      style={{
        position: 'absolute',
        left: 0,
        top: -offsetY,
        width: '100%',
        height: '100%',
        pointerEvents: 'none',
        overflow: 'visible',
      }}
    >
      <path
        d={path.d}
        fill="none"
        stroke={color}
        strokeWidth={PATH_STROKE_WIDTH}
        strokeDasharray={PATH_DASH}
      />
      <circle cx={path.from.x} cy={path.from.y} r={3} fill={color} />
      {state.kind !== 'idle' && state.snap ? (
        <circle cx={path.to.x} cy={path.to.y} r={5} fill="white" stroke={color} strokeWidth={2} />
      ) : null}
    </svg>
  )
}
