import { isPagePresented, type PagePresentationFocus } from '../../shared/page-presentation'
import type { ProjectedSceneEntity } from '../../shared/scene-projection'
import { pageChromeItem, type ChromeCanvasItem } from '../shared/chromeItemDraw'

/** One canvas item as `CanvasItemSurface` paints it. */
export interface CanvasItemDraw {
  item: ChromeCanvasItem
  /** Paint the item's border and device shell. */
  chrome: boolean
  /** The page whose latest frame fills the content rect; null for a file. */
  pageId: string | null
}

/**
 * Every presented page and device-framed file in scene order, focused page
 * last, each carrying its own chrome and texture. Painting an item whole
 * before the next is what lets a page stacked above another cover that page's
 * bezel as well as its content.
 *
 * A 'fill' page draws edge to edge with no border or bezel. Files are
 * context, so they drop out while a focus session hides context (ADR 0021).
 */
export function orderCanvasItemDraws(
  entities: readonly ProjectedSceneEntity[],
  focus: PagePresentationFocus,
): CanvasItemDraw[] {
  const fillPageId = focus.mode === 'fill' ? focus.pageId : null
  const draws: CanvasItemDraw[] = []
  let focused: CanvasItemDraw | null = null

  for (const entity of entities) {
    if (entity.kind === 'file') {
      if (focus.showsContext && entity.showDeviceFrame) {
        draws.push({ item: entity, chrome: true, pageId: null })
      }
      continue
    }
    if (entity.kind !== 'page' || !isPagePresented(entity.id, focus)) continue
    const isFill = entity.id === fillPageId
    const draw: CanvasItemDraw = {
      item: pageChromeItem(entity, isFill ? { showDeviceFrame: false } : undefined),
      chrome: !isFill,
      pageId: entity.id,
    }
    if (entity.id === focus.pageId) focused = draw
    else draws.push(draw)
  }

  if (focused) draws.push(focused)
  return draws
}
