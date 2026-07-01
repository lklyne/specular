import React, { memo } from 'react'
import type { CanvasScenePageEntity, CanvasSceneFileEntity } from '../../shared/types'
import {
  entityContentScreenRect,
  entityVisualScreenRect,
  type Camera,
} from '../../shared/coords'
import {
  CUSTOM_SHELL_CORNER_RADIUS,
  CUSTOM_SHELL_SCREEN_CORNER_RADIUS,
  DEVICE_CATALOG,
  contentCornerRadiusForDevice,
} from '../../shared/device-catalog'

type BorderItem = CanvasScenePageEntity | CanvasSceneFileEntity

export const PageBorderLayer = memo(function PageBorderLayer({
  pages,
  fileEntities,
  camera,
  offsetY = 0,
}: {
  pages: CanvasScenePageEntity[]
  fileEntities?: CanvasSceneFileEntity[]
  camera: Camera
  offsetY?: number
}) {
  const items: BorderItem[] = [...pages, ...(fileEntities ?? []).filter((f) => f.showDeviceFrame)]
  return (
    <>
      {items.map((page) => {
        // Inner content border
        const content = entityContentScreenRect(page, camera)
        const cx = content.screenX
        const cy = content.screenY - offsetY
        const cw = content.screenWidth
        const ch = content.screenHeight

        // Outer page border
        const outer = entityVisualScreenRect(page, camera)
        const fx = outer.screenX
        const fy = outer.screenY - offsetY
        const fw = outer.screenWidth
        const fh = outer.screenHeight

        const hasShell = page.showDeviceFrame

        // SVG device shell handles its own borders
        if (hasShell && 'useSvgDeviceShell' in page && page.useSvgDeviceShell) return null
        const dev = hasShell && page.deviceId ? DEVICE_CATALOG.get(page.deviceId) : null
        const displayZoom = page.width > 0 ? cw / page.width : 1
        const innerRadius = hasShell
          ? (dev
              ? contentCornerRadiusForDevice(page.deviceId!, page.deviceOrientation ?? 'portrait')
              : CUSTOM_SHELL_SCREEN_CORNER_RADIUS) * displayZoom
          : 0
        const outerRadius = hasShell
          ? (dev?.cornerRadius ?? CUSTOM_SHELL_CORNER_RADIUS) * displayZoom
          : 0

        const borderStyle = '1px solid var(--surface-device-border)'

        // Both borders render for every page. For non-device pages the outer
        // and inner rects coincide (contentScreen* falls back to screen*), so
        // the two 1px borders simply overlap — visually identical to one border.
        // When a device shell is active the outer border traces the bezel edge
        // and the inner border traces the content viewport cutout.
        return (
          <React.Fragment key={page.id}>
            {/* Outer page border */}
            <div
              className="pointer-events-none absolute"
              style={{
                left: fx - 1,
                top: fy - 1,
                width: fw + 2,
                height: fh + 2,
                borderRadius: outerRadius,
                border: borderStyle,
              }}
            />
            {/* Inner content border */}
            <div
              className="pointer-events-none absolute"
              style={{
                left: cx - 1,
                top: cy - 1,
                width: cw + 2,
                height: ch + 2,
                borderRadius: innerRadius,
                border: borderStyle,
              }}
            />
          </React.Fragment>
        )
      })}
    </>
  )
})
