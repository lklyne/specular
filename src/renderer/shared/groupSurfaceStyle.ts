import { resolveCanvasColor } from '../../shared/canvas-colors'
import type { CanvasSceneGroupEntity } from '../../shared/types'

export function groupSurfaceStyle(group: CanvasSceneGroupEntity, isDark: boolean) {
  if (!group.color) {
    return {
      borderColor: isDark ? 'rgba(161,161,170,0.25)' : 'rgba(113,113,122,0.25)',
      background: isDark ? 'rgba(39,39,42,0.35)' : 'rgba(244,244,245,0.45)',
    }
  }

  const resolvedColor = resolveCanvasColor(group.color, { palette: 'vivid' })
  return {
    borderColor: isDark
      ? `color-mix(in srgb, ${resolvedColor} 72%, #f4f4f5)`
      : `color-mix(in srgb, ${resolvedColor} 78%, #a16207)`,
    background: `color-mix(in srgb, ${resolvedColor} ${isDark ? '20%' : '30%'}, transparent)`,
  }
}
