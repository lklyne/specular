import type { ProjectedLayoutData } from '../../shared/scene-projection'
import { useMemo } from 'react'
import { focusContext } from '../../shared/focus-context'
import type { PagePresentationInputs } from '../../shared/page-presentation'
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
  const focus = focusContext(layoutData)
  // Eye off during focus: group backgrounds are context, hidden, never dimmed (ADR 0021).
  const hideContext = focus.active && !focus.showsContext
  const chromeGroups = useMemo(
    () => (hideContext ? [] : (layoutData.groups ?? [])),
    [hideContext, layoutData.groups],
  )
  const presentationInputs: PagePresentationInputs = useMemo(
    () => ({
      focus: {
        pageId: focus.pageId,
        mode: focus.mode,
        annotationsVisible: focus.data?.annotationsVisible ?? false,
        active: focus.active,
      },
    }),
    [focus.pageId, focus.mode, focus.active, focus.data?.annotationsVisible],
  )
  const canvasItemDraws = useMemo(
    () => orderCanvasItemDraws(layoutData.entities, presentationInputs),
    [layoutData.entities, presentationInputs],
  )
  return { canvasItemDraws, chromeGroups }
}
