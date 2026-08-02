/**
 * Renderer-side layout override for content-sized bounds (CONTEXT.md).
 *
 * A sticky's height is only knowable after its text lays out, so the renderer
 * measures it. Routing that measurement through main and waiting for the
 * broadcast back would leave every layer that reads stored bounds — selection
 * outline, resize handles, group bounds — a frame behind the card itself.
 *
 * So the measurement patches the layout here, before any layer reads it: the
 * card, the outline, and the handles all take the same number from the same
 * object in the same frame. Main still receives the measurement for
 * persistence and hit-testing; rendering just no longer waits on it.
 *
 * Same shape as `reorderPreviewLayout` / `gapPreviewLayout`: returns null when
 * it has nothing to say, so callers can chain with `??`.
 */

import type { LayoutUpdateData } from '../../shared/types'

export function contentHeightLayout(
  layout: LayoutUpdateData,
  heights: Map<string, number>,
): LayoutUpdateData | null {
  if (heights.size === 0) return null
  let changed = false
  const entities = layout.entities.map((entity) => {
    const height = heights.get(entity.id)
    if (height === undefined || height === entity.height) return entity
    changed = true
    return { ...entity, height, screenHeight: height * layout.zoom }
  })
  return changed ? { ...layout, entities } : null
}
