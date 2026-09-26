import type { ProjectedLayoutData } from '../../shared/scene-projection'
import { useMemo } from 'react'
import { focusContext } from '../../shared/focus-context'
import type { PagePresentationFocus } from '../../shared/page-presentation'
import { orderCanvasItemDraws, type CanvasItemDraw } from './canvasItemDrawOrder'

/**
 * The slices canvas-bg's chrome layers draw, already filtered for focus.
 * Each slice is memoized so the memoized layers receive stable array refs and
 * skip re-rendering when nothing they draw moved. Inline .filter() in JSX
 * would defeat React.memo (#265).
 */
export function useChromeSlices(
  layoutData: ProjectedLayoutData,
): {
  /** Pages and device-framed files in paint order — see `orderCanvasItemDraws`. */
  canvasItemDraws: CanvasItemDraw[]
  chromeGroups: NonNullable<ProjectedLayoutData['groups']>
} {
  const { active, pageId, mode, showsContext } = focusContext(layoutData)
  // Eye off during focus: group backgrounds are context, hidden, never dimmed (ADR 0021).
  const chromeGroups = useMemo(
    () => (showsContext ? (layoutData.groups ?? []) : []),
    [showsContext, layoutData.groups],
  )
  const focus: PagePresentationFocus = useMemo(
    () => ({ active, pageId, mode, showsContext }),
    [active, pageId, mode, showsContext],
  )
  const canvasItemDraws = useMemo(
    () => orderCanvasItemDraws(layoutData.entities, focus),
    [layoutData.entities, focus],
  )
  return { canvasItemDraws, chromeGroups }
}
