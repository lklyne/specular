import type { LayoutUpdateData } from '../../shared/types'

export function unionScreenBounds(
  pages: LayoutUpdateData['entities'],
  selectedEntityIds: string[],
) {
  const selectedPages = pages.filter((page) =>
    selectedEntityIds.includes(page.id),
  )
  if (!selectedPages.length) return null

  const left = Math.min(...selectedPages.map((page) => page.screenX))
  const top = Math.min(...selectedPages.map((page) => page.screenY))
  const right = Math.max(
    ...selectedPages.map((page) => page.screenX + page.screenWidth),
  )
  const bottom = Math.max(
    ...selectedPages.map((page) => page.screenY + page.screenHeight),
  )

  return {
    left,
    top,
    width: right - left,
    height: bottom - top,
  }
}
