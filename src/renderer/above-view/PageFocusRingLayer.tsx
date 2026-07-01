import type { CanvasSceneFileEntity, CanvasScenePageEntity } from '../../shared/types'
import {
  entityContentScreenRect,
  entityVisualScreenRect,
  type Camera,
} from '../../shared/coords'
import {
  CUSTOM_SHELL_CORNER_RADIUS,
  DEVICE_CATALOG,
} from '../../shared/device-catalog'

type RingTarget = CanvasScenePageEntity | CanvasSceneFileEntity

/** Focused-page accent ring. Lives in aboveView so it paints above page
 *  WCVs (and any neighboring page that overlaps the page's outer rect):
 *  4px outside the outer page border, 2px accent + 2px translucent halo.
 *  originY is canvasOrigin.y so window-coord pages land at the right
 *  WCV-local y. */
export function PageFocusRingLayer({
  pages,
  fileEntities,
  focusedPageId,
  camera,
  originY,
}: {
  pages: CanvasScenePageEntity[]
  fileEntities?: CanvasSceneFileEntity[]
  focusedPageId: string | null
  camera: Camera
  originY: number
}) {
  if (!focusedPageId) return null
  const items: RingTarget[] = [
    ...pages,
    ...(fileEntities ?? []).filter((f) => f.showDeviceFrame),
  ]
  const target = items.find((item) => item.id === focusedPageId)
  if (!target) return null

  const outer = entityVisualScreenRect(target, camera)
  const fx = outer.screenX
  const fy = outer.screenY - originY
  const fw = outer.screenWidth
  const fh = outer.screenHeight

  const hasShell = target.showDeviceFrame
  const dev = hasShell && target.deviceId ? DEVICE_CATALOG.get(target.deviceId) : null
  const cw = entityContentScreenRect(target, camera).screenWidth
  const displayZoom = target.width > 0 ? cw / target.width : 1
  const outerRadius = hasShell
    ? (dev?.cornerRadius ?? CUSTOM_SHELL_CORNER_RADIUS) * displayZoom
    : 0

  return (
    <div
      className="pointer-events-none absolute"
      style={{
        left: fx - 4,
        top: fy - 4,
        width: fw + 8,
        height: fh + 8,
        borderRadius: outerRadius + 3,
        boxShadow:
          '0 0 0 2px var(--accent), 0 0 0 4px color-mix(in srgb, var(--accent) 25%, transparent)',
      }}
    />
  )
}
