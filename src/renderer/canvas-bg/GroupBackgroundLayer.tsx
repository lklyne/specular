import { memo } from 'react'
import type { CanvasSceneGroupEntity } from '../../shared/types'
import { groupSurfaceStyle } from '../shared/groupSurfaceStyle'

export const GroupBackgroundLayer = memo(function GroupBackgroundLayer({
  groups,
  isDark,
}: {
  groups: CanvasSceneGroupEntity[]
  isDark: boolean
}) {
  if (!groups.length) return null

  return (
    <>
      {groups.map((group) => {
        const surfaceStyle = groupSurfaceStyle(group, isDark)
        return (
          <div
            key={group.id}
            className="pointer-events-none absolute"
            style={{
              left: group.screenX,
              top: group.screenY,
              width: group.screenWidth,
              height: group.screenHeight,
              borderRadius: 2,
              background: surfaceStyle.background,
            }}
          />
        )
      })}
    </>
  )
})
