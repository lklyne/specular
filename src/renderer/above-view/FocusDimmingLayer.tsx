import { FOCUS_DIMMED_ITEM_OPACITY, focusContext } from '../../shared/focus-context'
import type { CanvasScenePageEntity, LayoutUpdateData } from '../../shared/types'

export function FocusDimmingLayer({
  layoutData,
  isDark,
}: {
  layoutData: LayoutUpdateData
  isDark: boolean
}) {
  const focus = focusContext(layoutData)
  if (!focus.dimsOtherPages) return null

  const pages = layoutData.entities.filter(
    (entity): entity is CanvasScenePageEntity =>
      entity.kind === 'page' && entity.id !== focus.pageId,
  )
  if (!pages.length) return null

  const originY = layoutData.canvasOrigin.y
  const background = isDark
    ? 'color-mix(in srgb, var(--surface-canvas) 92%, black)'
    : 'color-mix(in srgb, var(--surface-canvas) 92%, white)'

  return (
    <>
      {pages.map((page) => (
        <div
          key={`focus-dim-${page.id}`}
          data-focus-dim-id={page.id}
          className="pointer-events-none absolute"
          style={{
            left: page.screenX,
            top: page.screenY - originY,
            width: page.screenWidth,
            height: page.screenHeight,
            background,
            opacity: 1 - FOCUS_DIMMED_ITEM_OPACITY,
          }}
        />
      ))}
    </>
  )
}
