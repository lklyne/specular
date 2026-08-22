/**
 * Canvas 2D rendering of page chrome: the 1px page/content borders, the
 * frozen-page raster that stands in for a parked WebContentsView, and the
 * device shell (bezel donut, strokes, island, home indicator).
 *
 * Drawn in screen space from the live camera transform on every pan/zoom
 * tick, so strokes are always rendered at display scale and stay crisp at any
 * zoom, instead of riding the CSS scene transform as a bitmap-scaled DOM layer.
 */
import {
  CUSTOM_SHELL_CORNER_RADIUS,
  CUSTOM_SHELL_SCREEN_CORNER_RADIUS,
  DEVICE_CATALOG,
  contentCornerRadiusForDevice,
} from '../../shared/device-catalog'
import type { SceneCameraTransform } from '../../shared/scene-camera-transform'
import { squirclePath } from './squirclePath'

export interface ChromeCanvasItem {
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
  useSvgDeviceShell?: boolean
  width: number
}

interface ItemGeometry {
  shellX: number
  shellY: number
  shellW: number
  shellH: number
  contentX: number
  contentY: number
  contentW: number
  contentH: number
  displayZoom: number
  outerRadius: number
  innerRadius: number
}

/** Map payload-space screen geometry through the live scene transform. */
function liveGeometry(
  item: ChromeCanvasItem,
  t: SceneCameraTransform,
): ItemGeometry {
  const shellX = t.x + t.scale * item.screenX
  const shellY = t.y + t.scale * item.screenY
  const shellW = t.scale * item.screenWidth
  const shellH = t.scale * item.screenHeight
  const contentX = t.x + t.scale * (item.contentScreenX ?? item.screenX)
  const contentY = t.y + t.scale * (item.contentScreenY ?? item.screenY)
  const contentW = t.scale * (item.contentScreenWidth ?? item.screenWidth)
  const contentH = t.scale * (item.contentScreenHeight ?? item.screenHeight)
  const displayZoom = item.width > 0 ? contentW / item.width : 1
  const dev = item.deviceId ? DEVICE_CATALOG.get(item.deviceId) : null
  const hasShell = item.showDeviceFrame === true
  const orientation = item.deviceOrientation ?? 'portrait'
  const outerRadius = hasShell
    ? (dev?.cornerRadius ?? CUSTOM_SHELL_CORNER_RADIUS) * displayZoom
    : 0
  const innerRadius = hasShell
    ? (dev
        ? contentCornerRadiusForDevice(item.deviceId!, orientation)
        : CUSTOM_SHELL_SCREEN_CORNER_RADIUS) * displayZoom
    : 0
  return {
    shellX,
    shellY,
    shellW,
    shellH,
    contentX,
    contentY,
    contentW,
    contentH,
    displayZoom,
    outerRadius,
    innerRadius,
  }
}

/** Snap a 1px stroke rect so the stroke centerline sits on a half-device-pixel
 * boundary. Edges land on device pixels and the line renders crisp. */
function snapStrokeRect(
  x: number,
  y: number,
  w: number,
  h: number,
  dpr: number,
): { x: number; y: number; w: number; h: number } {
  const snap = (v: number) => (Math.round(v * dpr - 0.5) + 0.5) / dpr
  const sx = snap(x)
  const sy = snap(y)
  return { x: sx, y: sy, w: snap(x + w) - sx, h: snap(y + h) - sy }
}

function strokeRoundedRect(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  radius: number,
  dpr: number,
): void {
  const r = snapStrokeRect(x, y, w, h, dpr)
  ctx.beginPath()
  ctx.roundRect(r.x, r.y, r.w, r.h, Math.max(0, radius))
  ctx.stroke()
}

function squircle2D(
  x: number,
  y: number,
  w: number,
  h: number,
  radius: number,
): Path2D {
  return new Path2D(squirclePath(x, y, w, h, radius))
}

/**
 * The content cutout. A circular arc, not a squircle: the live
 * WebContentsView's corner comes from setBorderRadius, which only rounds
 * circularly, and the cutout must match it exactly or the mismatch shows as
 * a bezel-coloured sliver along the corner.
 */
function contentCutout2D(
  x: number,
  y: number,
  w: number,
  h: number,
  radius: number,
): Path2D {
  const p = new Path2D()
  p.roundRect(x, y, w, h, Math.max(0, radius))
  return p
}

/**
 * The two 1px `--surface-device-border` borders every page carries: the outer
 * border traces the page/shell bounds, the inner traces the content viewport.
 * For non-device pages the rects coincide and overlap into a single border.
 * Border-box divs sat 1px outside the bounds; the stroke centerline at -0.5
 * reproduces that ring.
 */
function drawItemBorders(
  ctx: CanvasRenderingContext2D,
  g: ItemGeometry,
  borderColor: string,
  dpr: number,
  /** Device shells are squircles; the outer stroke must trace the same curve
   * as the bezel fill or the two drift apart along the corner. */
  squircleOuter: boolean,
): void {
  ctx.strokeStyle = borderColor
  ctx.lineWidth = 1
  if (squircleOuter) {
    const r = snapStrokeRect(g.shellX - 0.5, g.shellY - 0.5, g.shellW + 1, g.shellH + 1, dpr)
    ctx.stroke(squircle2D(r.x, r.y, r.w, r.h, g.outerRadius))
  } else {
    strokeRoundedRect(
      ctx,
      g.shellX - 0.5,
      g.shellY - 0.5,
      g.shellW + 1,
      g.shellH + 1,
      g.outerRadius,
      dpr,
    )
  }
  strokeRoundedRect(
    ctx,
    g.contentX - 0.5,
    g.contentY - 0.5,
    g.contentW + 1,
    g.contentH + 1,
    g.innerRadius,
    dpr,
  )
}

/**
 * The frozen-page raster, clipped to the content viewport's corner radius.
 * Painted last, where the live WebContentsView sits in the native stack: it
 * occludes the inner border ring and the bezel's drop shadow, which a shadowed
 * donut casts into its own cutout as well as outward.
 */
function drawItemSnapshot(
  ctx: CanvasRenderingContext2D,
  g: ItemGeometry,
  bitmap: ImageBitmap,
): void {
  ctx.save()
  ctx.clip(contentCutout2D(g.contentX, g.contentY, g.contentW, g.contentH, g.innerRadius))
  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(bitmap, g.contentX, g.contentY, g.contentW, g.contentH)
  ctx.restore()
}

/** The device shell: squircle bezel donut with drop shadow, edge strokes,
 * top highlight, and phone/tablet decorations. Mirrors DeviceShellLayer. */
function drawItemShell(
  ctx: CanvasRenderingContext2D,
  item: ChromeCanvasItem,
  g: ItemGeometry,
  isDark: boolean,
  bezelColor: string,
  dpr: number,
): void {
  const dev = item.deviceId ? DEVICE_CATALOG.get(item.deviceId) : null
  const orientation = item.deviceOrientation ?? 'portrait'
  const isPhone = dev?.category === 'iphone'
  const isTablet = dev?.category === 'ipad'
  const dz = g.displayZoom
  const insetBottom = g.shellY + g.shellH - (g.contentY + g.contentH)

  const outerPath = squircle2D(g.shellX, g.shellY, g.shellW, g.shellH, g.outerRadius)
  const donut = new Path2D()
  donut.addPath(outerPath)
  donut.addPath(contentCutout2D(g.contentX, g.contentY, g.contentW, g.contentH, g.innerRadius))

  // Bezel fill + drop shadow. Canvas shadow params ignore the transform, so
  // scale them by dpr explicitly.
  ctx.save()
  ctx.shadowColor = isDark ? 'rgba(0,0,0,0.8)' : 'rgba(0,0,0,0.24)'
  ctx.shadowOffsetY = 16 * dz * dpr
  ctx.shadowBlur = 16 * dz * dpr
  ctx.fillStyle = bezelColor
  ctx.fill(donut, 'evenodd')
  ctx.restore()

  // Outer 1px ring (box-shadow spread ring in the DOM version).
  ctx.strokeStyle = isDark ? 'rgba(0,0,0,0.15)' : 'rgba(0,0,0,0.06)'
  ctx.lineWidth = 1
  ctx.stroke(outerPath)

  // Top-edge bezel highlight (inset 0 1px 0).
  ctx.save()
  ctx.clip(donut, 'evenodd')
  ctx.fillStyle = isDark ? 'rgba(255,255,255,0.04)' : 'rgba(255,255,255,0.5)'
  ctx.fillRect(g.shellX, g.shellY, g.shellW, 1)
  ctx.restore()

  // Content-edge ring, painted in the bezel just outside the cutout. The
  // live WebContentsView fills the cutout exactly, so anything drawn inside
  // it is covered.
  const ringW = 2
  ctx.strokeStyle = isDark ? 'rgba(255,255,255,0.12)' : 'rgba(0,0,0,0.12)'
  ctx.lineWidth = ringW
  ctx.stroke(
    contentCutout2D(
      g.contentX - ringW / 2,
      g.contentY - ringW / 2,
      g.contentW + ringW,
      g.contentH + ringW,
      g.innerRadius + ringW / 2,
    ),
  )
  ctx.lineWidth = 1

  const centerX = g.shellX + g.shellW / 2

  if (isPhone && dev && dev.screenCornerRadius > 0 && orientation === 'portrait') {
    ctx.fillStyle = isDark ? '#000' : '#1a1a1a'
    ctx.beginPath()
    ctx.roundRect(
      centerX - (126 * dz) / 2,
      g.contentY + 8 * dz,
      126 * dz,
      37 * dz,
      18.5 * dz,
    )
    ctx.fill()
  }

  if (isPhone || isTablet) {
    const indicatorW = isTablet ? 100 : orientation === 'portrait' ? 120 : 100
    const bottomInset = Math.max(
      4 * dz,
      insetBottom / 2 - (isTablet ? 3 : 4) * dz,
    )
    ctx.fillStyle = isPhone
      ? isDark
        ? 'rgba(255,255,255,0.25)'
        : 'rgba(0,0,0,0.15)'
      : isDark
        ? 'rgba(255,255,255,0.2)'
        : 'rgba(0,0,0,0.12)'
    ctx.beginPath()
    ctx.roundRect(
      centerX - (indicatorW * dz) / 2,
      g.shellY + g.shellH - bottomInset - 4 * dz,
      indicatorW * dz,
      4 * dz,
      2 * dz,
    )
    ctx.fill()
  }
}

export function drawChromeCanvas({
  canvas,
  pages,
  fileEntities,
  snapshots,
  transform,
  isDark,
  devicePixelRatio,
}: {
  canvas: HTMLCanvasElement
  pages: ChromeCanvasItem[]
  fileEntities: ChromeCanvasItem[]
  /** Frozen-page rasters by page id; a page with no entry draws no raster. */
  snapshots: ReadonlyMap<string, ImageBitmap>
  transform: SceneCameraTransform
  isDark: boolean
  devicePixelRatio: number
}): void {
  const dpr = Math.max(devicePixelRatio, 1)
  const width = canvas.clientWidth
  const height = canvas.clientHeight
  const targetWidth = Math.max(1, Math.ceil(width * dpr))
  const targetHeight = Math.max(1, Math.ceil(height * dpr))
  if (canvas.width !== targetWidth) canvas.width = targetWidth
  if (canvas.height !== targetHeight) canvas.height = targetHeight

  const ctx = canvas.getContext('2d')
  if (!ctx) return
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0)
  ctx.clearRect(0, 0, width, height)

  const styles = getComputedStyle(canvas)
  const borderColor =
    styles.getPropertyValue('--surface-device-border').trim() || '#d6d3d1'
  const bezelColor =
    styles.getPropertyValue('--surface-device').trim() || '#e7e5e4'

  const framedFiles = fileEntities.filter((f) => f.showDeviceFrame)
  // The SVG shell layer owns its pages entirely (borders included).
  const borderItems = [
    ...pages.filter((p) => !(p.showDeviceFrame && p.useSvgDeviceShell)),
    ...framedFiles,
  ]
  const shellItems = [
    ...pages.filter((p) => p.showDeviceFrame && !p.useSvgDeviceShell),
    ...framedFiles,
  ]

  // Three passes matching the native stacking order: all borders, then all
  // shells (the bezel fill covers the inner border ring on shell pages), then
  // the page rasters standing in for the live views on top.
  for (const item of borderItems) {
    drawItemBorders(ctx, liveGeometry(item, transform), borderColor, dpr, !!item.showDeviceFrame)
  }
  for (const item of shellItems) {
    drawItemShell(ctx, item, liveGeometry(item, transform), isDark, bezelColor, dpr)
  }
  if (snapshots.size > 0) {
    for (const page of pages) {
      const bitmap = snapshots.get(page.id)
      // A closed bitmap reports zero size; drawing it throws.
      if (bitmap && bitmap.width > 0) drawItemSnapshot(ctx, liveGeometry(page, transform), bitmap)
    }
  }
}
