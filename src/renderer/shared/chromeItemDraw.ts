/**
 * Canvas 2D drawing of one page/file item's chrome: the 1px page/content
 * borders, the frozen-page raster that stands in for a parked
 * WebContentsView, and the device shell (bezel donut, strokes, island,
 * home indicator). Pure per-item geometry and draw calls, shared by
 * canvas-bg (`chromeCanvasDraw.ts`) and above-view (`DragFreezeLayer`).
 *
 * Drawn in screen space at display scale on every tick, so strokes stay crisp
 * at any zoom instead of riding a scaled DOM layer.
 */
import {
  CUSTOM_SHELL_CORNER_RADIUS,
  CUSTOM_SHELL_SCREEN_CORNER_RADIUS,
  DEVICE_CATALOG,
  contentCornerRadiusForDevice,
} from '../../shared/device-catalog'
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

export interface ItemGeometry {
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

/** The two CSS custom properties every chrome pass reads for its border and
 *  bezel fill colors, resolved once per draw against a canvas already
 *  mounted in the themed surface tree. */
export function readChromeColors(canvas: HTMLCanvasElement): {
  borderColor: string
  bezelColor: string
} {
  const styles = getComputedStyle(canvas)
  return {
    borderColor: styles.getPropertyValue('--surface-device-border').trim() || '#d6d3d1',
    bezelColor: styles.getPropertyValue('--surface-device').trim() || '#e7e5e4',
  }
}

/** The shell/content rects and shell corner radii one item draws at. */
export function itemGeometry(item: ChromeCanvasItem): ItemGeometry {
  const shellX = item.screenX
  const shellY = item.screenY
  const shellW = item.screenWidth
  const shellH = item.screenHeight
  const contentX = item.contentScreenX ?? item.screenX
  const contentY = item.contentScreenY ?? item.screenY
  const contentW = item.contentScreenWidth ?? item.screenWidth
  const contentH = item.contentScreenHeight ?? item.screenHeight
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

/** Centerline rect and stroke width for a ~1px ring hugging the outside of
 * the given bounds. The stroke covers a whole number of device pixels flush
 * against the bounds, so none of it lands under the WebContentsView (or bezel
 * fill) that covers the bounds, at any dpr. */
function outsetStrokeRect(
  x: number,
  y: number,
  w: number,
  h: number,
  dpr: number,
): { x: number; y: number; w: number; h: number; lineWidth: number } {
  const lw = Math.max(1, Math.round(dpr))
  const left = Math.round(x * dpr)
  const top = Math.round(y * dpr)
  const right = Math.round((x + w) * dpr)
  const bottom = Math.round((y + h) * dpr)
  return {
    x: (left - lw / 2) / dpr,
    y: (top - lw / 2) / dpr,
    w: (right - left + lw) / dpr,
    h: (bottom - top + lw) / dpr,
    lineWidth: lw / dpr,
  }
}

/** Stroke the ring `outsetStrokeRect` describes, as a circular round-rect or
 * a squircle. Owns strokeStyle and lineWidth so no caller touches ctx stroke
 * state by hand. */
function strokeOutsetRing(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  w: number,
  h: number,
  radius: number,
  color: string,
  dpr: number,
  squircle: boolean,
): void {
  const r = outsetStrokeRect(x, y, w, h, dpr)
  // The centerline sits lineWidth/2 outside the bounds, so the corner radius
  // grows by the same amount to keep the ring concentric with the bounds'
  // corner curve; passing the radius through unchanged shifts the arc center
  // and opens a sub-pixel sliver between ring and fill at the diagonals.
  const outsetRadius = radius > 0 ? radius + r.lineWidth / 2 : 0
  ctx.strokeStyle = color
  ctx.lineWidth = r.lineWidth
  if (squircle) {
    ctx.stroke(squircle2D(r.x, r.y, r.w, r.h, outsetRadius))
  } else {
    ctx.beginPath()
    ctx.roundRect(r.x, r.y, r.w, r.h, outsetRadius)
    ctx.stroke()
  }
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
 * One item's chrome, minus its raster. The shell/plain split lives here so
 * canvas-bg and the drag-freeze layer can never disagree about which pass
 * owns an item's border.
 *
 * Either way the item ends in one 1px `--surface-device-border` ring hugging
 * the outside of its bounds, painted after the shell: the bezel's drop
 * shadow falls on the side/bottom edges but never the top, so a border
 * painted under it comes out shadow-darkened on three edges and reads
 * thicker there than on the top.
 */
export function drawItemChrome(
  ctx: CanvasRenderingContext2D,
  item: ChromeCanvasItem,
  g: ItemGeometry,
  isDark: boolean,
  bezelColor: string,
  borderColor: string,
  dpr: number,
): void {
  if (item.showDeviceFrame) {
    drawItemShell(ctx, item, g, isDark, bezelColor, dpr)
    // The border must trace the same squircle curve as the bezel fill or the
    // two drift apart along the corner.
    strokeOutsetRing(ctx, g.shellX, g.shellY, g.shellW, g.shellH, g.outerRadius, borderColor, dpr, true)
  } else {
    // A shell-less page projects identical shell and content rects, so this
    // single ring is both its page and content border.
    strokeOutsetRing(ctx, g.contentX, g.contentY, g.contentW, g.contentH, g.innerRadius, borderColor, dpr, false)
  }
}

/**
 * The frozen-page raster, clipped to the content viewport's corner radius.
 * Painted last, where the live WebContentsView sits in the native stack: it
 * occludes the inner border ring and the bezel's drop shadow, which a shadowed
 * donut casts into its own cutout as well as outward.
 */
export function drawItemSnapshot(
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
 * top highlight, and phone/tablet decorations. Mirrors SvgDeviceShellLayer.
 * The shell's border is `drawItemChrome`'s job, painted on top of this. */
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
  const ringW = 1.5
  ctx.strokeStyle = isDark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.12)'
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

  drawDeviceDecorations(ctx, g, isDark, {
    isPhone,
    isTablet,
    notch: isPhone && !!dev && dev.screenCornerRadius > 0 && orientation === 'portrait',
    orientation,
  })
}

/** Phone notch (portrait only) and the phone/tablet home indicator. */
function drawDeviceDecorations(
  ctx: CanvasRenderingContext2D,
  g: ItemGeometry,
  isDark: boolean,
  device: { isPhone: boolean; isTablet: boolean; notch: boolean; orientation: string },
): void {
  const { isPhone, isTablet, notch, orientation } = device
  if (!isPhone && !isTablet) return
  const dz = g.displayZoom
  const centerX = g.shellX + g.shellW / 2
  const insetBottom = g.shellY + g.shellH - (g.contentY + g.contentH)

  if (notch) {
    ctx.fillStyle = isDark ? '#000' : '#1a1a1a'
    ctx.beginPath()
    ctx.roundRect(centerX - (126 * dz) / 2, g.contentY + 8 * dz, 126 * dz, 37 * dz, 18.5 * dz)
    ctx.fill()
  }

  const indicatorW = isTablet ? 100 : orientation === 'portrait' ? 120 : 100
  const bottomInset = Math.max(4 * dz, insetBottom / 2 - (isTablet ? 3 : 4) * dz)
  const light = isPhone ? 'rgba(255,255,255,0.25)' : 'rgba(255,255,255,0.2)'
  const dark = isPhone ? 'rgba(0,0,0,0.15)' : 'rgba(0,0,0,0.12)'
  ctx.fillStyle = isDark ? light : dark
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
