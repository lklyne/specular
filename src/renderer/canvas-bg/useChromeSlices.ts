import type { ProjectedFileEntity, ProjectedLayoutData, ProjectedPageEntity } from '../../shared/scene-projection'
import { useMemo } from 'react'
import { focusContext } from '../../shared/focus-context'

/**
 * The entity slices the chrome layers draw, already filtered for focus.
 * Each slice is memoized so the memoized layers receive stable array refs and
 * skip re-rendering when nothing they draw moved. Inline .filter() in JSX
 * would defeat React.memo (#265).
 */
export function useChromeSlices(
  layoutData: ProjectedLayoutData,
  dragFrozenPageIds: ReadonlySet<string>,
): {
  chromePages: ProjectedPageEntity[]
  chromeFiles: ProjectedFileEntity[]
  svgDeviceShellPages: ProjectedPageEntity[]
  chromeGroups: NonNullable<ProjectedLayoutData['groups']>
} {
  const pageEntities = useMemo(
    () => layoutData.entities.filter((e): e is ProjectedPageEntity => e.kind === 'page'),
    [layoutData.entities],
  )
  const fileEntities = useMemo(
    () => layoutData.entities.filter((e): e is ProjectedFileEntity => e.kind === 'file'),
    [layoutData.entities],
  )
  const focus = focusContext(layoutData)
  // 'fill' focus is the browser-like mode: edge-to-edge, no border, no bezel.
  const fillPageId = focus.mode === 'fill' ? focus.pageId : null
  // Eye off during focus: only the focused page's chrome survives; all other
  // context (other pages, file frames, groups) is hidden, never dimmed (ADR 0021).
  const hideContext = focus.active && !focus.showsContext
  const chromePages = useMemo(() => {
    if (fillPageId) return pageEntities.filter((p) => p.id !== fillPageId)
    if (hideContext) return pageEntities.filter((p) => p.id === focus.pageId)
    return pageEntities
  }, [pageEntities, fillPageId, hideContext, focus.pageId])
  const chromeFiles = useMemo(
    () => (hideContext ? [] : fileEntities),
    [fileEntities, hideContext],
  )
  const svgDeviceShellPages = useMemo(
    // above-view's DragFreezeLayer draws a drag-frozen page's shell; this
    // DOM layer would otherwise draw it again at its stale position.
    () => chromePages.filter((f) => f.useSvgDeviceShell && !dragFrozenPageIds.has(f.id)),
    [chromePages, dragFrozenPageIds],
  )
  const chromeGroups = useMemo(
    () => (hideContext ? [] : (layoutData.groups ?? [])),
    [hideContext, layoutData.groups],
  )
  return { chromePages, chromeFiles, svgDeviceShellPages, chromeGroups }
}
