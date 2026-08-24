import { GRID_SIZE } from '../../shared/constants'
import { prepareScreenCanvas } from '../shared/screenCanvas'

const MIN_GRID_SPACING_PX = 8
const FULL_OPACITY_SPACING_PX = 18
const MAX_GRID_STEP_MULTIPLIER = 64

function devicePixelRatioOrOne(devicePixelRatio: number) {
  return Math.max(devicePixelRatio, 1)
}

function gridStepMultiplierForZoom(zoom: number) {
  let multiplier = 1
  while (
    GRID_SIZE * zoom * multiplier < MIN_GRID_SPACING_PX &&
    multiplier < MAX_GRID_STEP_MULTIPLIER
  ) {
    multiplier *= 2
  }
  return multiplier
}

function gridAlpha(spacing: number, isDark: boolean) {
  const minAlpha = isDark ? 0.56 : 0.52
  const alpha = spacing / FULL_OPACITY_SPACING_PX
  return Math.max(minAlpha, Math.min(1, alpha))
}

function buildCanvasGridMetrics({
  canvasOrigin,
  pan,
  zoom,
  isDark,
  devicePixelRatio,
}: {
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  isDark: boolean
  devicePixelRatio: number
}) {
  const stepMultiplier = gridStepMultiplierForZoom(zoom)
  const spacing = GRID_SIZE * zoom * stepMultiplier
  const originX = canvasOrigin.x + pan.x
  const originY = canvasOrigin.y + pan.y

  return {
    originX,
    originY,
    spacing,
    stepMultiplier,
    dotRadius: Math.max(
      0.6,
      Math.round(0.7 * devicePixelRatioOrOne(devicePixelRatio)) /
        devicePixelRatioOrOne(devicePixelRatio),
    ),
    alpha: gridAlpha(spacing, isDark),
  }
}

/**
 * One tile of the dot field, in device pixels, cached across frames. The dot
 * sits at the tile's centre so the repeat never clips it — a dot on the tile
 * origin would paint one quarter of itself and neighbouring tiles would supply
 * nothing for the other three.
 */
let cachedDotTile: { canvas: HTMLCanvasElement; key: string } | null = null

function gridDotTile(
  tileDevicePx: number,
  radiusDevicePx: number,
  color: string,
): HTMLCanvasElement | null {
  const key = `${tileDevicePx}:${radiusDevicePx}:${color}`
  if (cachedDotTile?.key === key) return cachedDotTile.canvas

  const canvas = document.createElement('canvas')
  canvas.width = tileDevicePx
  canvas.height = tileDevicePx
  const ctx = canvas.getContext('2d')
  if (!ctx) return null
  const centre = Math.round(tileDevicePx / 2)
  ctx.fillStyle = color
  ctx.beginPath()
  ctx.arc(centre, centre, radiusDevicePx, 0, Math.PI * 2)
  ctx.fill()
  cachedDotTile = { canvas, key }
  return canvas
}

export function buildCanvasGridStyle() {
  return {
    backgroundColor: 'var(--surface-canvas)',
  }
}

export function drawCanvasGrid({
  canvas,
  color,
  canvasOrigin,
  pan,
  zoom,
  isDark,
  devicePixelRatio,
}: {
  canvas: HTMLCanvasElement
  color: string
  canvasOrigin: { x: number; y: number }
  pan: { x: number; y: number }
  zoom: number
  isDark: boolean
  devicePixelRatio: number
}) {
  const prepared = prepareScreenCanvas(canvas, devicePixelRatioOrOne(devicePixelRatio))
  if (!prepared) return
  const { ctx, width, height, dpr } = prepared

  const metrics = buildCanvasGridMetrics({
    canvasOrigin,
    pan,
    zoom,
    isDark,
    devicePixelRatio: dpr,
  })

  const { spacing, originX, originY, dotRadius, alpha } = metrics
  if (!Number.isFinite(spacing) || spacing <= 0) return

  // A tile spanning a whole number of device pixels lands every dot on the
  // device grid, so the whole field stays crisp without rounding each dot's
  // centre by hand. Rounding the tile moves the spacing by well under a pixel.
  const tileDevicePx = Math.max(1, Math.round(spacing * dpr))
  const tile = gridDotTile(tileDevicePx, dotRadius * dpr, color)
  if (!tile) return
  const pattern = ctx.createPattern(tile, 'repeat')
  if (!pattern) return

  // The tile carries its dot at the centre, so the field is phased by the grid
  // origin less half a tile. Only the offset within one tile matters; the
  // repeat covers the rest.
  const tileCssPx = tileDevicePx / dpr
  const centreCssPx = Math.round(tileDevicePx / 2) / dpr
  const phase = (value: number) =>
    (((value - centreCssPx) % tileCssPx) + tileCssPx) % tileCssPx
  // The pattern draws one image pixel per user unit, and user units are CSS
  // pixels here, so it scales back down to device pixels before phasing.
  pattern.setTransform(
    new DOMMatrix().translate(phase(originX), phase(originY)).scale(1 / dpr),
  )

  ctx.fillStyle = pattern
  ctx.globalAlpha = alpha
  ctx.fillRect(0, 0, width, height)
  ctx.globalAlpha = 1
}
