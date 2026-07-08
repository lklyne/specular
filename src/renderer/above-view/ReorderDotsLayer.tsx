import { useMemo, useState } from 'react'
import type { LayoutUpdateData } from '../../shared/types'
import {
  REORDER_DOT_HOVER_RADIUS_PX,
  REORDER_DOT_VISUAL_RADIUS_PX,
} from '../../shared/canvas-hit-geometry'
import { reorderableDots } from '../../shared/reorderable-dots'
import { REARRANGE_COLOR } from '../canvas-bg/canvasBgConstants'

/**
 * Reorder dots (ADR 0015 D7). Paints the per-entity center dot that hosts the
 * reorder gesture — a geometric overlay (like edge anchors), not DOM buttons.
 * A thin pink ring at rest; a solid pink fill when the pointer is over the dot
 * itself (dot-local hover, not whole-entity hover).
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
}: {
  layoutData: LayoutUpdateData
}) {
  const { canvasOrigin, interaction } = layoutData
  // Dot-local hover: which dot the pointer is directly over. Whole-entity hover
  // no longer grows the dot — only the handle itself reacts.
  const [hoveredDotId, setHoveredDotId] = useState<string | null>(null)

  const reordering = interaction.kind === 'reordering-row' ? interaction : null

  const dots = useMemo(() => {
    if (layoutData.activeTool.kind !== 'select') return []
    // Show dots only at rest or while reordering; hide during drag/resize/
    // marquee/edit so they don't clutter an in-progress gesture.
    if (interaction.kind !== 'idle' && interaction.kind !== 'reordering-row') return []

    const eligible = reorderableDots(layoutData)
    if (!eligible.length) return []

    const out: Array<{ id: string; cx: number; cy: number }> = []
    for (const dot of eligible) {
      // The dragged entity is "lifted" — it floats as a ghost under the cursor
      // (Phase D), so it carries no dot.
      if (reordering && reordering.movingId === dot.id) continue
      out.push({
        id: dot.id,
        cx: dot.center.x,
        cy: dot.center.y - canvasOrigin.y,
      })
    }
    return out
  }, [interaction.kind, layoutData, canvasOrigin.y, reordering])

  if (!dots.length) return null

  return (
    <svg
      className="pointer-events-none absolute inset-0 h-full w-full overflow-visible"
      aria-hidden="true"
    >
      {dots.map((dot) => {
        const hovered = hoveredDotId === dot.id
        return (
          <g key={dot.id}>
            {hovered ? (
              // Hovered: solid pink fill, grown.
              <circle cx={dot.cx} cy={dot.cy} r={REORDER_DOT_HOVER_RADIUS_PX} fill={REARRANGE_COLOR} />
            ) : (
              // At rest: thick pink ring (hollow center).
              <circle
                cx={dot.cx}
                cy={dot.cy}
                r={REORDER_DOT_VISUAL_RADIUS_PX}
                fill="none"
                stroke={REARRANGE_COLOR}
                strokeWidth={2.5}
              />
            )}
            {/* Transparent hit target drives dot-local hover. Pointer-down still
                routes through the window-level router. */}
            <circle
              cx={dot.cx}
              cy={dot.cy}
              r={REORDER_DOT_HOVER_RADIUS_PX}
              fill="transparent"
              style={{ pointerEvents: 'all' }}
              onPointerEnter={() => setHoveredDotId(dot.id)}
              onPointerLeave={() =>
                setHoveredDotId((current) => (current === dot.id ? null : current))
              }
            />
          </g>
        )
      })}
    </svg>
  )
}
