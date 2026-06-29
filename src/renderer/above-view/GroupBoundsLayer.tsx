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

/**
 * Wraps the group-bound rectangles in a viewport transform so they live in
 * canvas-coordinate space. AboveView's WCV origin already sits at
 * `canvasOrigin.y` (the toolbar inset), so the translate omits that axis
 * — only `canvasOrigin.x` and `pan` apply. Matches `StickyViewportLayer`.
 */
function GroupViewportLayer({
  canvasOrigin,
  pan,
  zoom,
  children,
}: {
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  children: React.ReactNode
}) {
  return (
    <div
      className="pointer-events-none absolute left-0 top-0 origin-top-left"
      style={{
        ['--canvas-zoom' as string]: zoom,
        transform: `translate(${canvasOrigin.x + pan.x}px, ${pan.y}px) scale(${zoom})`,
      }}
    >
      {children}
    </div>
  )
}

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
    <GroupViewportLayer canvasOrigin={canvasOrigin} pan={pan} zoom={zoom}>
      {groups.map((group) => (
        <GroupBoundsItem
          key={group.id}
          group={group}
          isDark={isDark}
          inverseScale={inverseScale}
        />
      ))}
    </GroupViewportLayer>
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
