import { useMemo } from 'react'
import type { LayoutUpdateData } from '../../shared/types'
import {
  REORDER_DOT_HOVER_RADIUS_PX,
  REORDER_DOT_VISUAL_RADIUS_PX,
} from '../../shared/canvas-hit-geometry'
import { reorderableDots } from '../../shared/reorderable-dots'
import { selectionColor } from '../canvas-bg/canvasBgConstants'

/**
 * Reorder dots (ADR 0015 D7). Paints the per-entity center dot that hosts the
 * reorder gesture — a geometric overlay (like edge anchors), not DOM buttons.
 * Small at rest, larger when the entity or its managed group is hovered.
 *
 * Eligibility comes from the one shared `reorderableDots` selector — the same
 * source the hit-tester consumes — so the visible dot and the grabbable target
 * line up by construction (the union of the selection and managed doors).
 * Suppressed during any non-reorder interaction and while a non-select tool is
 * active, matching popup suppression rules. During a reorder drag the dragged
 * entity's dot is hidden; the row entities reflow live to their previewed slots
 * (App feeds this layer the reorder-preview layout), so the dots ride along with
 * the siblings as they make room — no separate insertion line needed.
 */
export function ReorderDotsLayer({
  layoutData,
  isDark,
}: {
  layoutData: LayoutUpdateData
  isDark: boolean
}) {
  const color = selectionColor(isDark)
  const { canvasOrigin, entities, interaction } = layoutData

  const reordering = interaction.kind === 'reordering-row' ? interaction : null

  const dots = useMemo(() => {
    if (layoutData.viewMode !== 'canvas') return []
    if (layoutData.activeTool.kind !== 'select') return []
    // Show dots only at rest or while reordering; hide during drag/resize/
    // marquee/edit so they don't clutter an in-progress gesture.
    if (interaction.kind !== 'idle' && interaction.kind !== 'reordering-row') return []

    const eligible = reorderableDots(layoutData)
    if (!eligible.length) return []

    // Grow a dot when its entity is hovered, or when the hovered entity is the
    // managed-row group that contains it.
    const hoverId = layoutData.hover?.id ?? null
    const hoveredGroupChildren = new Set<string>()
    if (hoverId) {
      const hovered = entities.find((e) => e.id === hoverId)
      if (hovered?.kind === 'group' && hovered.managedLayout && hovered.layoutMode === 'row') {
        for (const childId of hovered.entityIds) hoveredGroupChildren.add(childId)
      }
    }

    const out: Array<{ id: string; cx: number; cy: number; r: number }> = []
    for (const dot of eligible) {
      // The dragged entity is "lifted" — it floats as a ghost under the cursor
      // (Phase D), so it carries no dot.
      if (reordering && reordering.movingId === dot.id) continue
      const grown = hoverId === dot.id || hoveredGroupChildren.has(dot.id)
      out.push({
        id: dot.id,
        cx: dot.center.x,
        cy: dot.center.y - canvasOrigin.y,
        r: grown ? REORDER_DOT_HOVER_RADIUS_PX : REORDER_DOT_VISUAL_RADIUS_PX,
      })
    }
    return out
  }, [
    entities,
    interaction.kind,
    layoutData,
    canvasOrigin.y,
    reordering,
  ])

  if (!dots.length) return null

  return (
    <svg
      className="pointer-events-none absolute inset-0 h-full w-full overflow-visible"
      aria-hidden="true"
    >
      {dots.map((dot) => (
        <circle key={dot.id} cx={dot.cx} cy={dot.cy} r={dot.r} fill={color} opacity={0.85} />
      ))}
    </svg>
  )
}
