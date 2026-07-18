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
import { selectionColor } from '../canvas-bg/canvasBgConstants'
import { groupSurfaceStyle } from '../shared/groupSurfaceStyle'
import { CanvasViewportLayer } from './CanvasViewportLayer'

export const GroupBoundsLayer = memo(function GroupBoundsLayer({
  groups,
  isDark,
  zoom,
  canvasOrigin,
  pan,
  dropTargetGroupId,
}: {
  groups: CanvasSceneGroupEntity[]
  isDark: boolean
  zoom: number
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  dropTargetGroupId: string | null
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
          isDropTarget={group.id === dropTargetGroupId}
        />
      ))}
    </CanvasViewportLayer>
  )
})

function GroupBoundsItem({
  group,
  isDark,
  inverseScale,
  isDropTarget,
}: {
  group: CanvasSceneGroupEntity
  isDark: boolean
  inverseScale: number
  isDropTarget: boolean
}) {
  const surfaceStyle = groupSurfaceStyle(group, isDark)
  const dropTargetPadding = isDropTarget ? 2 * inverseScale : 0

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
        className="absolute"
        style={{
          left: -dropTargetPadding,
          top: -dropTargetPadding,
          width: group.width + dropTargetPadding * 2,
          height: group.height + dropTargetPadding * 2,
          borderRadius: isDropTarget ? 0 : 2 * inverseScale,
          border: `${(isDropTarget ? 2 : 1.5) * inverseScale}px solid ${
            isDropTarget ? selectionColor(isDark) : surfaceStyle.borderColor
          }`,
        }}
      />
    </div>
  )
}
