import { useMemo } from 'react'
import type { LayoutUpdateData } from '../../shared/types'
import { GAP_HANDLE_BAR_THICKNESS_PX } from '../../shared/canvas-hit-geometry'
import { collectGapHandleZones } from '../../shared/gap-handles'
import { selectionColor } from '../canvas-bg/canvasBgConstants'

/**
 * Gap handles (ADR 0015 Milestone 2). The strips between a managed group's
 * adjacent children that host the gap-resize drag. Geometry and eligibility
 * come from the one shared `collectGapHandleZones` selector — the same source
 * the hit-tester consumes — so the cursor affordance and the grabbable target
 * line up by construction (visible when the managed group or a child is
 * selected, matching the reorder dots' managed door).
 *
 * The strips carry `pointerEvents: 'all'` purely for the col-resize /
 * row-resize hover cursor (the EdgeLayer precedent) — the pointerdown itself
 * is still classified by the router's window-level hit-test, never by DOM
 * handlers here. A center bar lights up on hover and stays lit on every strip
 * of the active group during the drag.
 */
export function GapHandlesLayer({
  layoutData,
  isDark,
}: {
  layoutData: LayoutUpdateData
  isDark: boolean
}) {
  const color = selectionColor(isDark)
  const { canvasOrigin, interaction } = layoutData

  const resizingGroupId = interaction.kind === 'resizing-gap' ? interaction.groupId : null

  const zones = useMemo(() => {
    if (layoutData.activeTool.kind !== 'select') return []
    // Show at rest or during the gap drag itself; hide during any other
    // in-progress gesture, matching the reorder dots' suppression rules.
    if (interaction.kind !== 'idle' && interaction.kind !== 'resizing-gap') return []
    return collectGapHandleZones(layoutData)
  }, [layoutData, interaction.kind])

  if (!zones.length) return null

  return (
    <div className="pointer-events-none absolute inset-0" aria-hidden="true">
      {zones.map((zone) => {
        const active = zone.groupId === resizingGroupId
        const horizontalBar = zone.axis === 'y'
        return (
          <div
            key={`${zone.groupId}-${zone.index}`}
            className="group absolute flex items-center justify-center"
            style={{
              left: zone.rect.x,
              top: zone.rect.y - canvasOrigin.y,
              width: zone.rect.width,
              height: zone.rect.height,
              cursor: zone.axis === 'x' ? 'col-resize' : 'row-resize',
              pointerEvents: 'all',
            }}
          >
            <div
              className={active ? '' : 'opacity-0 group-hover:opacity-70'}
              style={{
                width: horizontalBar ? '100%' : GAP_HANDLE_BAR_THICKNESS_PX,
                height: horizontalBar ? GAP_HANDLE_BAR_THICKNESS_PX : '100%',
                borderRadius: GAP_HANDLE_BAR_THICKNESS_PX / 2,
                background: color,
                opacity: active ? 0.7 : undefined,
                transition: 'opacity 80ms ease',
              }}
            />
          </div>
        )
      })}
    </div>
  )
}
