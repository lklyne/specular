import type { ProjectedPageEntity } from '../../shared/scene-projection'
import { isPagePresented, type PagePresentationInputs } from '../../shared/page-presentation'

/**
 * The pages `PageTextureSurface` draws, and the order it draws them in:
 * every presented page in scene order, with the focused page moved last so
 * its texture paints over whatever sits behind it (the fill/device/fit modes
 * all have the focused page occluding its neighbors).
 */
export function orderTexturePages(
  pages: ProjectedPageEntity[],
  presentation: PagePresentationInputs,
): ProjectedPageEntity[] {
  const presented = pages.filter((page) => isPagePresented(page.id, presentation))
  const focusedId = presentation.focus.pageId
  if (!focusedId) return presented
  const rest = presented.filter((page) => page.id !== focusedId)
  const focused = presented.filter((page) => page.id === focusedId)
  if (focused.length === 0) return presented
  return [...rest, ...focused]
}
