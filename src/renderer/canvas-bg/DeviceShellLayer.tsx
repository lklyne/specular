import type { CanvasScenePageEntity, CanvasSceneFileEntity } from '../../shared/types'
import {
  CUSTOM_SHELL_CORNER_RADIUS,
  CUSTOM_SHELL_SCREEN_CORNER_RADIUS,
  DEVICE_CATALOG,
  contentCornerRadiusForDevice,
} from '../../shared/device-catalog'

/** Shared shape for anything that can render a device shell. */
interface DeviceShellItem {
  id: string
  screenX: number
  screenY: number
  screenWidth: number
  screenHeight: number
  contentScreenX?: number
  contentScreenY?: number
  contentScreenWidth?: number
  contentScreenHeight?: number
  deviceId?: string | null
  deviceOrientation?: 'portrait' | 'landscape'
  showDeviceFrame?: boolean
  width: number
}

export function DeviceShellLayer({
  pages,
  fileEntities,
  isDark,
}: {
  pages: CanvasScenePageEntity[]
  fileEntities?: CanvasSceneFileEntity[]
  isDark: boolean
}) {
  const framedPages: DeviceShellItem[] = pages.filter((f) => f.showDeviceFrame)
  const framedFiles: DeviceShellItem[] = (fileEntities ?? []).filter((f) => f.showDeviceFrame)

  if (!framedPages.length && !framedFiles.length) return null

  const renderItem = (page: DeviceShellItem) => {
        const dev = page.deviceId ? DEVICE_CATALOG.get(page.deviceId) : null

        const orientation = page.deviceOrientation ?? 'portrait'

        const contentX = page.contentScreenX ?? page.screenX
        const contentY = page.contentScreenY ?? page.screenY
        const contentW = page.contentScreenWidth ?? page.screenWidth
        const contentH = page.contentScreenHeight ?? page.screenHeight

        // Shell outer rect is screenX/Y/Width/Height (already outer bounds)
        const shellX = page.screenX
        const shellY = page.screenY
        const shellW = page.screenWidth
        const shellH = page.screenHeight

        // Insets in screen space
        const insetTop = contentY - shellY
        const insetLeft = contentX - shellX
        const insetBottom = shellY + shellH - (contentY + contentH)
        const insetRight = shellX + shellW - (contentX + contentW)

        // Derive zoom from screen vs logical content width
        const displayZoom = page.width > 0 ? contentW / page.width : 1
        const outerRadius = (dev?.cornerRadius ?? CUSTOM_SHELL_CORNER_RADIUS) * displayZoom
        const innerRadius = dev
          ? contentCornerRadiusForDevice(page.deviceId!, orientation) * displayZoom
          : CUSTOM_SHELL_SCREEN_CORNER_RADIUS * displayZoom

        const isPhone = dev?.category === 'iphone'
        const isTablet = dev?.category === 'ipad'

        const bezelColor = 'var(--surface-device)'
        const bezelHighlight = isDark
          ? 'rgba(255,255,255,0.04)'
          : 'rgba(255,255,255,0.5)'

        return (
          <div
            key={page.id}
            className="pointer-events-none absolute"
            style={{
              left: shellX,
              top: shellY,
              width: shellW,
              height: shellH,
              filter: isDark
                ? `drop-shadow(0 ${16 * displayZoom}px ${16 * displayZoom}px rgba(0,0,0,0.8))`
                : `drop-shadow(0 ${16 * displayZoom}px ${16 * displayZoom}px rgba(0,0,0,0.24))`,
            }}
          >
            {/* Outer bezel */}
            <div
              className="absolute inset-0"
              style={{
                borderRadius: outerRadius,
                background: bezelColor,
                boxShadow: isDark
                  ? `inset 0 1px 0 ${bezelHighlight}, 0 0 0 1px rgba(0,0,0,0.15)`
                  : `inset 0 1px 0 ${bezelHighlight}, 0 0 0 1px rgba(0,0,0,0.06)`,
              }}
            />

            {/* Content cutout (transparent hole) */}
            <div
              className="absolute overflow-hidden"
              style={{
                left: insetLeft,
                top: insetTop,
                width: contentW,
                height: contentH,
                borderRadius: innerRadius,
                boxShadow: isDark
                  ? `inset 0 0 0 1px rgba(255,255,255,0.06)`
                  : `inset 0 0 0 1px rgba(0,0,0,0.06)`,
              }}
            />

            {/* Mask so bezel doesn't cover content — use clip-path with hole */}
            <div
              className="absolute inset-0"
              style={{
                borderRadius: outerRadius,
                background: bezelColor,
                clipPath: `polygon(
                  0% 0%, 100% 0%, 100% 100%, 0% 100%, 0% 0%,
                  ${pct(insetLeft, shellW)} ${pct(insetTop, shellH)},
                  ${pct(insetLeft, shellW)} ${pct(insetTop + contentH, shellH)},
                  ${pct(insetLeft + contentW, shellW)} ${pct(insetTop + contentH, shellH)},
                  ${pct(insetLeft + contentW, shellW)} ${pct(insetTop, shellH)},
                  ${pct(insetLeft, shellW)} ${pct(insetTop, shellH)}
                )`,
                boxShadow: isDark
                  ? `inset 0 1px 0 ${bezelHighlight}`
                  : `inset 0 1px 0 ${bezelHighlight}`,
              }}
            />

            {/* Phone-specific: Dynamic Island (edge-to-edge phones only) */}
            {isPhone &&
              dev.screenCornerRadius > 0 &&
              orientation === 'portrait' && (
                <div
                  className="absolute left-1/2 -translate-x-1/2"
                  style={{
                    top: insetTop + 8 * displayZoom,
                    width: 126 * displayZoom,
                    height: 37 * displayZoom,
                    borderRadius: 18.5 * displayZoom,
                    background: isDark ? '#000' : '#1a1a1a',
                  }}
                />
              )}

            {/* Phone-specific: Home indicator */}
            {isPhone && (
              <div
                className="absolute left-1/2 -translate-x-1/2"
                style={{
                  bottom: Math.max(
                    4 * displayZoom,
                    insetBottom / 2 - 4 * displayZoom,
                  ),
                  width: (orientation === 'portrait' ? 120 : 100) * displayZoom,
                  height: 4 * displayZoom,
                  borderRadius: 2 * displayZoom,
                  background: isDark
                    ? 'rgba(255,255,255,0.25)'
                    : 'rgba(0,0,0,0.15)',
                }}
              />
            )}

            {/* Tablet-specific: subtle home indicator */}
            {isTablet && (
              <div
                className="absolute left-1/2 -translate-x-1/2"
                style={{
                  bottom: Math.max(
                    4 * displayZoom,
                    insetBottom / 2 - 3 * displayZoom,
                  ),
                  width: 100 * displayZoom,
                  height: 4 * displayZoom,
                  borderRadius: 2 * displayZoom,
                  background: isDark
                    ? 'rgba(255,255,255,0.2)'
                    : 'rgba(0,0,0,0.12)',
                }}
              />
            )}
          </div>
        )
  }

  return (
    <>
      {framedPages.map(renderItem)}
      {framedFiles.map(renderItem)}
    </>
  )
}

function pct(value: number, total: number): string {
  return `${((value / total) * 100).toFixed(4)}%`
}
