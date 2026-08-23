import type { ProjectedLayoutData } from '../../shared/scene-projection'
import { memo, useEffect, useMemo, useState } from 'react'
import {
  REORDER_DOT_VISUAL_RADIUS_PX,
  reorderHandleHitPx,
} from '../../shared/canvas-hit-geometry'
import { reorderableDots } from '../../shared/reorderable-dots'
import { REARRANGE_COLOR } from '../canvas-bg/canvasBgConstants'

/**
 * Reorder dots (ADR 0015 D7). Paints the per-entity center dot that hosts the
 * reorder gesture — a geometric overlay (like edge anchors), not DOM buttons.
 * A thin pink ring at rest; a solid pink fill when the pointer is over the ring
 * itself (dot-local hover, not whole-entity hover). The grown circle is drawn at
 * exactly the hit-tester's grab radius, so the hover state is an honest picture
 * of where a press starts a reorder — you arm it on the small ring and lose it
 * on the grown edge.
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
export const ReorderDotsLayer = memo(function ReorderDotsLayer({
  layoutData,
}: {
  layoutData: ProjectedLayoutData
}) {
  const { canvasOrigin, interaction } = layoutData
  // Dot-local hover: which dot the pointer is directly over. Whole-entity hover
  // no longer grows the dot — only the handle itself reacts.
  const [hoveredDotId, setHoveredDotId] = useState<string | null>(null)

  const reordering = interaction.kind === 'reordering-row' ? interaction : null

  // The dragged dot is unmounted for the duration of the drag, so it never gets
  // a pointerleave — clear hover on both edges of the gesture or it stays lit
  // after a drop that lands away from the dot.
  useEffect(() => setHoveredDotId(null), [reordering !== null])

  const dots = useMemo(() => {
    if (layoutData.activeTool.kind !== 'select') return []
    // Show dots only at rest or while reordering; hide during drag/resize/
    // marquee/edit so they don't clutter an in-progress gesture.
    if (interaction.kind !== 'idle' && interaction.kind !== 'reordering-row') return []

    const eligible = reorderableDots(layoutData)
    if (!eligible.length) return []

    const out: Array<{ id: string; cx: number; cy: number; hitRadius: number }> = []
    for (const dot of eligible) {
      // The dragged entity is "lifted" — it floats as a ghost under the cursor
      // (Phase D), so it carries no dot.
      if (reordering && reordering.movingId === dot.id) continue
      out.push({
        id: dot.id,
        cx: dot.center.x,
        cy: dot.center.y - canvasOrigin.y,
        // Mirrors the hit-tester's square exactly, so the ring only lights up
        // where a press would actually start a reorder.
        hitRadius: reorderHandleHitPx(dot.size.width, dot.size.height) / 2,
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
              <circle cx={dot.cx} cy={dot.cy} r={dot.hitRadius} fill={REARRANGE_COLOR} />
            ) : (
              // At rest: thick pink ring (hollow center).
              <circle
                cx={dot.cx}
                cy={dot.cy}
                r={Math.min(REORDER_DOT_VISUAL_RADIUS_PX, dot.hitRadius)}
                fill="none"
                stroke={REARRANGE_COLOR}
                strokeWidth={2.5}
              />
            )}
            {/* The small ring itself arms hover; the grown circle it becomes is
                then the honest picture of the grabbable area. Pointer-down still
                routes through the window-level router. */}
            <circle
              cx={dot.cx}
              cy={dot.cy}
              r={hovered ? dot.hitRadius : Math.min(REORDER_DOT_VISUAL_RADIUS_PX, dot.hitRadius)}
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
})
