import type { CanvasScenePageEntity, LayoutUpdateData } from '../../shared/types'
import {
  CUSTOM_SHELL_CORNER_RADIUS,
  CUSTOM_SHELL_SCREEN_CORNER_RADIUS,
  DEVICE_CATALOG,
  contentCornerRadiusForDevice,
} from '../../shared/device-catalog'
import { PageBorderLayer } from '../canvas-bg/PageBorderLayer'
import { SvgDeviceShellLayer } from '../canvas-bg/SvgDeviceShellLayer'
import { squirclePath } from '../canvas-bg/squirclePath'

export function FocusedPageFrameLayer({
  layoutData,
  isDark,
}: {
  layoutData: LayoutUpdateData
  isDark: boolean
}) {
  const focusedPageId = layoutData.focusPresentation?.pageId ?? null
  if (!focusedPageId) return null

  const focusedPage = layoutData.entities.find(
    (entity): entity is CanvasScenePageEntity =>
      entity.kind === 'page' && entity.id === focusedPageId,
  )
  if (!focusedPage) return null

  const originY = layoutData.canvasOrigin.y

  return (
    <>
      {focusedPage.showDeviceFrame && focusedPage.useSvgDeviceShell ? (
        <SvgDeviceShellLayer pages={[focusedPage]} isDark={isDark} offsetY={originY} />
      ) : null}
      {focusedPage.showDeviceFrame && !focusedPage.useSvgDeviceShell ? (
        <FocusedDeviceShellOverlay page={focusedPage} isDark={isDark} originY={originY} />
      ) : null}
      {!focusedPage.showDeviceFrame ? (
        <PageBorderLayer pages={[focusedPage]} offsetY={originY} />
      ) : null}
    </>
  )
}

function FocusedDeviceShellOverlay({
  page,
  isDark,
  originY,
}: {
  page: CanvasScenePageEntity
  isDark: boolean
  originY: number
}) {
  const dev = page.deviceId ? DEVICE_CATALOG.get(page.deviceId) : null
  const orientation = page.deviceOrientation ?? 'portrait'

  const contentX = page.contentScreenX ?? page.screenX
  const contentY = (page.contentScreenY ?? page.screenY) - originY
  const contentW = page.contentScreenWidth ?? page.screenWidth
  const contentH = page.contentScreenHeight ?? page.screenHeight

  const shellX = page.screenX
  const shellY = page.screenY - originY
  const shellW = page.screenWidth
  const shellH = page.screenHeight

  const insetTop = contentY - shellY
  const insetLeft = contentX - shellX
  const insetBottom = shellY + shellH - (contentY + contentH)

  const displayZoom = page.width > 0 ? contentW / page.width : 1
  const outerRadius = (dev?.cornerRadius ?? CUSTOM_SHELL_CORNER_RADIUS) * displayZoom
  const innerRadius = dev
    ? contentCornerRadiusForDevice(page.deviceId!, orientation) * displayZoom
    : CUSTOM_SHELL_SCREEN_CORNER_RADIUS * displayZoom

  const isPhone = dev?.category === 'iphone'
  const isTablet = dev?.category === 'ipad'
  const pad = 2
  const svgW = shellW + pad * 2
  const svgH = shellH + pad * 2
  const ox = pad
  const oy = pad
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
      <path
        d={outerPath + innerPath}
        fillRule="evenodd"
        style={{ fill: 'var(--surface-device)' }}
      />
      <path
        d={outerPath}
        fill="none"
        style={{ stroke: 'var(--surface-device-border)' }}
        strokeWidth={1}
      />
      <path
        d={squirclePath(ox + insetLeft, oy + insetTop, contentW, contentH, innerRadius, 'cw')}
        fill="none"
        style={{ stroke: 'var(--surface-device-border)' }}
        strokeWidth={1}
      />
      {isPhone && dev && dev.screenCornerRadius > 0 && orientation === 'portrait' ? (
        <rect
          x={ox + shellW / 2 - (126 * displayZoom) / 2}
          y={oy + insetTop + 8 * displayZoom}
          width={126 * displayZoom}
          height={37 * displayZoom}
          rx={18.5 * displayZoom}
          fill={isDark ? '#000' : '#1a1a1a'}
        />
      ) : null}
      {isPhone ? (
        <rect
          x={ox + shellW / 2 - ((orientation === 'portrait' ? 120 : 100) * displayZoom) / 2}
          y={oy + shellH - Math.max(4 * displayZoom, insetBottom / 2 + 4 * displayZoom)}
          width={(orientation === 'portrait' ? 120 : 100) * displayZoom}
          height={4 * displayZoom}
          rx={2 * displayZoom}
          fill={isDark ? 'rgba(255,255,255,0.25)' : 'rgba(0,0,0,0.15)'}
        />
      ) : null}
      {isTablet ? (
        <rect
          x={ox + shellW / 2 - (100 * displayZoom) / 2}
          y={oy + shellH - Math.max(4 * displayZoom, insetBottom / 2 + 3 * displayZoom)}
          width={100 * displayZoom}
          height={4 * displayZoom}
          rx={2 * displayZoom}
          fill={isDark ? 'rgba(255,255,255,0.2)' : 'rgba(0,0,0,0.12)'}
        />
      ) : null}
    </svg>
  )
}
