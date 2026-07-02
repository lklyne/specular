/**
 * GroupBoundsLayer — group bound borders. Mounted in aboveView so a group
 * containing a page keeps its border visible above the page. The filled
 * group background lives in canvas-bg, below page WebContentsViews.
 *
 * Purely visual (`pointer-events: none` end-to-end) — selection / drag /
 * double-click-to-enter-group are all driven by `useCanvasPointerRouter`
 * against the layout snapshot, not by direct DOM events on this surface.
 */
import { memo } from 'react'
import type { CanvasSceneGroupEntity } from '../../shared/types'
import { groupSurfaceStyle } from '../shared/groupSurfaceStyle'
import { CanvasViewportLayer } from './CanvasViewportLayer'

export const GroupBoundsLayer = memo(function GroupBoundsLayer({
  groups,
  isDark,
  zoom,
  canvasOrigin,
  pan,
}: {
  groups: CanvasSceneGroupEntity[]
  isDark: boolean
  zoom: number
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
}) {
  if (!groups.length) return null
  const inverseScale = 1 / zoom

  return (
    <CanvasViewportLayer canvasOrigin={canvasOrigin} pan={pan} zoom={zoom}>
      {groups.map((group) => (
        <GroupBoundsItem
          key={group.id}
          group={group}
          isDark={isDark}
          inverseScale={inverseScale}
        />
      ))}
    </CanvasViewportLayer>
  )
})

function GroupBoundsItem({
  group,
  isDark,
  inverseScale,
}: {
  group: CanvasSceneGroupEntity
  isDark: boolean
  inverseScale: number
}) {
  const surfaceStyle = groupSurfaceStyle(group, isDark)

  return (
    <div
      className="absolute"
      style={{
        left: group.canvasX,
        top: group.canvasY,
        width: group.width,
        height: group.height,
        overflow: 'visible',
        pointerEvents: 'none',
      }}
    >
      <div
        className="absolute inset-0"
        style={{
          borderRadius: 2 * inverseScale,
          border: `${1.5 * inverseScale}px solid ${surfaceStyle.borderColor}`,
          transition: 'border-color 120ms ease',
        }}
      />
    </div>
  )
}
