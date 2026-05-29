import { useMemo } from 'react'
import type { LayoutUpdateData } from '../../shared/types'
import {
  REORDER_DOT_HOVER_RADIUS_PX,
  REORDER_DOT_VISUAL_RADIUS_PX,
} from '../../shared/canvas-hit-geometry'
import { CLUSTER_HORIZONTAL_GUTTER } from '../../shared/constants'
import { selectionColor } from '../canvas-bg/canvasBgConstants'

/**
 * Auto-layout reorder dots (ADR 0015 Phase 4). Paints the per-child center dot
 * that hosts the reorder gesture — a geometric overlay (like edge anchors), not
 * DOM buttons. Small at rest, larger when the child or its group is hovered.
 *
 * Mirrors the `reorder-handle` hit-test gating exactly (managed-row child whose
 * group or self is selected), so the visible dot and the grabbable target line
 * up. Suppressed during any non-reorder interaction and while a non-select tool
 * is active, matching popup suppression rules. During a reorder drag the dragged
 * child's dot is hidden and an insertion line marks the live drop slot.
 */
export function ReorderDotsLayer({
  layoutData,
  isDark,
}: {
  layoutData: LayoutUpdateData
  isDark: boolean
}) {
  const color = selectionColor(isDark)
  const { zoom, pan, canvasOrigin, entities, interaction } = layoutData

  const reordering = interaction.kind === 'reordering-child' ? interaction : null

  const dots = useMemo(() => {
    if (layoutData.viewMode !== 'canvas') return []
    if (layoutData.activeTool.kind !== 'select') return []
    // Show dots only at rest or while reordering; hide during drag/resize/
    // marquee/edit so they don't clutter an in-progress gesture.
    if (interaction.kind !== 'idle' && interaction.kind !== 'reordering-child') return []

    const childToGroup = new Map<string, string>()
    for (const e of entities) {
      if (e.kind === 'group' && e.managedLayout && e.layoutMode === 'row') {
        for (const childId of e.entityIds) childToGroup.set(childId, e.id)
      }
    }
    if (!childToGroup.size) return []

    const selected = new Set(layoutData.selectedEntityIds)
    const hoverId = layoutData.hover?.id ?? null
    const out: Array<{ id: string; cx: number; cy: number; r: number }> = []
    for (const e of entities) {
      if (e.kind === 'group') continue
      const groupId = childToGroup.get(e.id)
      if (!groupId) continue
      if (layoutData.selectedGroupId !== groupId && !selected.has(e.id)) continue
      // The dragged child is "lifted" — its dot is replaced by the drop line.
      if (reordering && reordering.childId === e.id) continue
      const grown = hoverId === e.id || hoverId === groupId
      out.push({
        id: e.id,
        cx: e.screenX + e.screenWidth / 2,
        cy: e.screenY - canvasOrigin.y + e.screenHeight / 2,
        r: grown ? REORDER_DOT_HOVER_RADIUS_PX : REORDER_DOT_VISUAL_RADIUS_PX,
      })
    }
    return out
  }, [
    entities,
    interaction.kind,
    layoutData.activeTool.kind,
    layoutData.hover?.id,
    layoutData.selectedEntityIds,
    layoutData.selectedGroupId,
    layoutData.viewMode,
    canvasOrigin.y,
    reordering,
  ])

  // Insertion line: vertical bar at the drop slot of the dragged child's row.
  const insertionLine = useMemo(() => {
    if (!reordering) return null
    const group = entities.find((e) => e.kind === 'group' && e.id === reordering.groupId)
    if (!group) return null
    const childIds = new Set(group.kind === 'group' ? group.entityIds : [])
    const others = entities
      .filter((e) => e.kind !== 'group' && childIds.has(e.id) && e.id !== reordering.childId)
      .sort((a, b) => a.screenX - b.screenX)
    if (!others.length) return null

    const top = Math.min(...others.map((e) => e.screenY - canvasOrigin.y))
    const bottom = Math.max(...others.map((e) => e.screenY - canvasOrigin.y + e.screenHeight))
    const gapHalf = (CLUSTER_HORIZONTAL_GUTTER * zoom) / 2
    const index = Math.max(0, Math.min(reordering.dropIndex, others.length))
    const x =
      index < others.length
        ? others[index].screenX - gapHalf
        : others[others.length - 1].screenX + others[others.length - 1].screenWidth + gapHalf
    return { x, top, bottom }
  }, [reordering, entities, canvasOrigin.y, zoom])

  if (!dots.length && !insertionLine) return null

  return (
    <svg
      className="pointer-events-none absolute inset-0 h-full w-full overflow-visible"
      aria-hidden="true"
    >
      {dots.map((dot) => (
        <circle key={dot.id} cx={dot.cx} cy={dot.cy} r={dot.r} fill={color} opacity={0.85} />
      ))}
      {insertionLine ? (
        <line
          x1={insertionLine.x}
          y1={insertionLine.top}
          x2={insertionLine.x}
          y2={insertionLine.bottom}
          stroke={color}
          strokeWidth={2}
          vectorEffect="non-scaling-stroke"
        />
      ) : null}
    </svg>
  )
}
