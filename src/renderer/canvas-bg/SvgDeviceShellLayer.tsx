/**
 * SVG-based device shell renderer (experimental, option B).
 *
 * Uses a single SVG <path> with fill-rule="evenodd" to draw the device bezel
 * as a donut shape (outer squircle minus inner content cutout). Borders are
 * SVG strokes, phone/tablet decorations are SVG <rect> elements.
 *
 * Advantages over the CSS div-based DeviceShellLayer:
 * - No clip-path polygon hack — corners are true cubic Bezier squircles
 * - Single draw call for the bezel fill instead of three composited divs
 * - Fewer DOM nodes per device page
 *
 * Known issue: the top border stroke is clipped in some configurations.
 * Off by default — the CSS `DeviceShellLayer` is the active renderer and draws
 * squircle corners via `-electron-corner-smoothing`.
 *
 * Toggle via the per-page `useSvgDeviceShell` metadata flag (checkbox in
 * the right details panel, currently commented out).
 */

import { memo } from 'react'
import type { CanvasScenePageEntity } from '../../shared/types'
import {
  DEVICE_CATALOG,
  contentCornerRadiusForDevice,
} from '../../shared/device-catalog'
import { squirclePath } from '../shared/squirclePath'

export const SvgDeviceShellLayer = memo(function SvgDeviceShellLayer({
  pages,
  isDark,
}: {
  pages: CanvasScenePageEntity[]
  isDark: boolean
}) {
  const framedPages = pages.filter((f) => f.showDeviceFrame)

  if (!framedPages.length) return null

  return (
    <>
      {framedPages.map((page) => {
        const dev = page.deviceId ? DEVICE_CATALOG.get(page.deviceId) : null
        if (!dev) return null // SVG shell only supports catalog devices for now

        const orientation = page.deviceOrientation ?? 'portrait'

        // Inner content bounds (the web viewport area)
        const contentX = page.contentScreenX ?? page.screenX
        const contentY = page.contentScreenY ?? page.screenY
        const contentW = page.contentScreenWidth ?? page.screenWidth
        const contentH = page.contentScreenHeight ?? page.screenHeight

        // Outer shell bounds (device bezel outer edge)
        const shellX = page.screenX
        const shellY = page.screenY
        const shellW = page.screenWidth
        const shellH = page.screenHeight

        // Bezel insets (space between shell and content)
        const insetTop = contentY - shellY
        const insetLeft = contentX - shellX
        const insetBottom = shellY + shellH - (contentY + contentH)

        // Scale device catalog radii to screen space
        const displayZoom = page.width > 0 ? contentW / page.width : 1
        const outerRadius = dev.cornerRadius * displayZoom
        const innerRadius =
          contentCornerRadiusForDevice(page.deviceId!, orientation) * displayZoom

        const isPhone = dev.category === 'iphone'
        const isTablet = dev.category === 'ipad'

        // Padding around the shell so border strokes aren't clipped
        const pad = 2
        const svgW = shellW + pad * 2
        const svgH = shellH + pad * 2

        // All paths are offset by `pad` to sit within the SVG viewport
        const ox = pad
        const oy = pad

        // Outer shell path (clockwise) and inner cutout (counter-clockwise)
        // Combined with evenodd fill to create the bezel donut shape
        const outerPath = squirclePath(ox, oy, shellW, shellH, outerRadius, 'cw')
        const innerPath = squirclePath(
          ox + insetLeft,
          oy + insetTop,
          contentW,
          contentH,
          innerRadius,
          'ccw',
        )

        const shadowBlur = 16 * displayZoom

        return (
          <svg
            key={page.id}
            className="pointer-events-none absolute"
            style={{
              left: shellX - pad,
              top: shellY - pad,
              filter: isDark
                ? `drop-shadow(0 ${shadowBlur}px ${shadowBlur}px rgba(0,0,0,0.8))`
                : `drop-shadow(0 ${shadowBlur}px ${shadowBlur}px rgba(0,0,0,0.24))`,
            }}
            width={svgW}
            height={svgH}
            viewBox={`0 0 ${svgW} ${svgH}`}
            overflow="visible"
          >
            {/* Bezel donut: outer shell minus inner content cutout */}
            <path
              d={outerPath + innerPath}
              fillRule="evenodd"
              style={{ fill: 'var(--surface-device)' }}
            />

            {/* Outer border stroke */}
            <path
              d={outerPath}
              fill="none"
              style={{ stroke: 'var(--surface-device-border)' }}
              strokeWidth={1}
            />

            {/* Inner content border stroke */}
            <path
              d={squirclePath(
                ox + insetLeft,
                oy + insetTop,
                contentW,
                contentH,
                innerRadius,
                'cw',
              )}
              fill="none"
              style={{ stroke: 'var(--surface-device-border)' }}
              strokeWidth={1}
            />

            {/* Subtle highlight along the top edge of the bezel */}
            <path
              d={outerPath}
              fill="none"
              stroke={isDark ? 'rgba(255,255,255,0.04)' : 'rgba(255,255,255,0.5)'}
              strokeWidth={1}
              style={{ clipPath: `inset(0 0 ${shellH - 1}px 0)` }}
            />

            {/* Phone: Dynamic Island (edge-to-edge phones only) */}
            {isPhone && dev.screenCornerRadius > 0 && orientation === 'portrait' && (
              <rect
                x={ox + shellW / 2 - (126 * displayZoom) / 2}
                y={oy + insetTop + 8 * displayZoom}
                width={126 * displayZoom}
                height={37 * displayZoom}
                rx={18.5 * displayZoom}
                fill={isDark ? '#000' : '#1a1a1a'}
              />
            )}

            {/* Phone: Home indicator */}
            {isPhone && (
              <rect
                x={ox + shellW / 2 - ((orientation === 'portrait' ? 120 : 100) * displayZoom) / 2}
                y={oy + shellH - Math.max(4 * displayZoom, insetBottom / 2 + 4 * displayZoom)}
                width={(orientation === 'portrait' ? 120 : 100) * displayZoom}
                height={4 * displayZoom}
                rx={2 * displayZoom}
                fill={isDark ? 'rgba(255,255,255,0.25)' : 'rgba(0,0,0,0.15)'}
              />
            )}

            {/* Tablet: Home indicator */}
            {isTablet && (
              <rect
                x={ox + shellW / 2 - (100 * displayZoom) / 2}
                y={oy + shellH - Math.max(4 * displayZoom, insetBottom / 2 + 3 * displayZoom)}
                width={100 * displayZoom}
                height={4 * displayZoom}
                rx={2 * displayZoom}
                fill={isDark ? 'rgba(255,255,255,0.2)' : 'rgba(0,0,0,0.12)'}
              />
            )}
          </svg>
        )
      })}
    </>
  )
})
