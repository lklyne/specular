import { memo } from 'react'
import type { CanvasSceneGroupEntity } from '../../shared/types'
import { entityVisualScreenRect, type Camera } from '../../shared/coords'
import { groupSurfaceStyle } from '../shared/groupSurfaceStyle'

export const GroupBackgroundLayer = memo(function GroupBackgroundLayer({
  groups,
  isDark,
  camera,
}: {
  groups: CanvasSceneGroupEntity[]
  isDark: boolean
  camera: Camera
}) {
  if (!groups.length) return null

  return (
    <>
      {groups.map((group) => {
        const surfaceStyle = groupSurfaceStyle(group, isDark)
        const b = entityVisualScreenRect(group, camera)
        return (
          <div
            key={group.id}
            className="pointer-events-none absolute"
            style={{
              left: b.screenX,
              top: b.screenY,
              width: b.screenWidth,
              height: b.screenHeight,
              borderRadius: 2,
              background: surfaceStyle.background,
            }}
          />
        )
      })}
    </>
  )
})
