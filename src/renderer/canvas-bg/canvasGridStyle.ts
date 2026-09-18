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
let cachedDotTile: {
  canvas: HTMLCanvasElement
  centreDevicePx: number
  key: string
} | null = null

function gridDotTile(
  tileDevicePx: number,
  radiusDevicePx: number,
  color: string,
): { canvas: HTMLCanvasElement; centreDevicePx: number } | null {
  const key = `${tileDevicePx}:${radiusDevicePx}:${color}`
  if (cachedDotTile?.key === key) return cachedDotTile

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
  cachedDotTile = { canvas, centreDevicePx: centre, key }
  return cachedDotTile
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

  // The tile raster spans a whole number of device pixels so the cached dot
  // stays crisp; the pattern transform scales it to the exact spacing, so the
  // repeat period carries no rounding error to accumulate across the window.
  const tileDevicePx = Math.max(1, Math.round(spacing * dpr))
  const tile = gridDotTile(tileDevicePx, dotRadius * dpr, color)
  if (!tile) return
  const pattern = ctx.createPattern(tile.canvas, 'repeat')
  if (!pattern) return

  // The tile carries its dot at the centre, so the field is phased by the grid
  // origin less that offset. Only the offset within one repeat matters; the
  // repeat covers the rest.
  const rasterToCssPx = spacing / tileDevicePx
  const centreCssPx = tile.centreDevicePx * rasterToCssPx
  const phase = (value: number) =>
    (((value - centreCssPx) % spacing) + spacing) % spacing
  pattern.setTransform(
    new DOMMatrix()
      .translate(phase(originX), phase(originY))
      .scale(rasterToCssPx),
  )

  ctx.fillStyle = pattern
  ctx.globalAlpha = alpha
  ctx.fillRect(0, 0, width, height)
  ctx.globalAlpha = 1
}
